//! « Projet privé », its files (ESC-140), through the core: what each harness is told to refuse an
//! agent of a project beside a private one, and an agent of the private one, on its fake CLI. The
//! real harnesses at the end, by hand.

use crate::core::AgentOptions;
use crate::core_tests::{harness, repo, Harness};
use crate::engine::codex::{FENCE_PLAN_PROFILE, FENCE_PROFILE};
use crate::model::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// A project of the folder `<test dir>/<name>/repo`, private or not, as « Projet privé » saves it.
async fn project_in(h: &Harness, name: &str, private: bool) -> (Project, PathBuf) {
    let r = repo(&h.dir.join(name));
    let p = h
        .core
        .create_project(
            &r.to_string_lossy(),
            name,
            "oklch(0.7 0.1 200)",
            false,
            None,
            true,
        )
        .await
        .unwrap();
    if private {
        let mut projects = h.core.projects.write();
        projects.iter_mut().find(|x| x.id == p.id).unwrap().private = true;
    }
    (p, r)
}

/// `open`, not private, and `vault`, private.
async fn two_projects(h: &Harness) -> ((Project, PathBuf), (Project, PathBuf)) {
    let open = project_in(h, "open", false).await;
    let vault = project_in(h, "vault", true).await;
    (open, vault)
}

fn read_json(file: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap()
}

fn lines(file: &Path) -> Vec<Value> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The rules of Claude Code that refuse its tools `folder` (`fence::claude_settings`).
fn claude_rules(folder: &Path) -> [String; 2] {
    let r = crate::fence::claude_rule_path(&folder.to_string_lossy());
    [format!("Read({r}/**)"), format!("Edit({r}/**)")]
}

/// The one settings file of the last launch of the fake `claude` in `cwd`: Claude Code reads the
/// last `--settings` only.
fn claude_settings(h: &Harness, cwd: &Path) -> Value {
    let argv = h.launches(cwd).pop().expect("a launch");
    let files: Vec<&String> = argv
        .windows(2)
        .filter(|w| w[0] == "--settings")
        .map(|w| &w[1])
        .collect();
    assert_eq!(files.len(), 1, "{argv:?}");
    read_json(Path::new(files[0].as_str()))
}

fn denies(settings: &Value, rule: &str) -> bool {
    settings["permissions"]["deny"]
        .as_array()
        .is_some_and(|d| d.iter().any(|r| r == rule))
}

#[tokio::test]
async fn claude_code_is_refused_the_private_projects_folders_and_its_own_agents_the_others() {
    let h = harness("esc140-claude");
    let ((open, open_dir), (vault, vault_dir)) = two_projects(&h).await;
    let a = h.core.create_agent(&open.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&a).await.unwrap();
    let s = claude_settings(&h, &open_dir);
    for rule in claude_rules(&vault_dir) {
        assert!(denies(&s, &rule), "{rule} {s:#}");
    }
    // Escouade's state, which holds every project.
    let state = crate::fence::claude_rule_path(&h.core.data.state_file().to_string_lossy());
    assert!(denies(&s, &format!("Read({state})")), "{s:#}");
    for rule in claude_rules(&open_dir) {
        assert!(!denies(&s, &rule), "{rule}");
    }
    // Escouade's server kept out of it all the same (the switch is off): in this one file.
    assert_eq!(s["deniedMcpServers"][0]["serverName"], "escouade");

    // An agent of the private project: the other one refused.
    let b = h.core.create_agent(&vault.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&b).await.unwrap();
    let s = claude_settings(&h, &vault_dir);
    for rule in claude_rules(&open_dir) {
        assert!(denies(&s, &rule), "{rule} {s:#}");
    }
    for rule in claude_rules(&vault_dir) {
        assert!(!denies(&s, &rule), "{rule}");
    }
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_project_made_private_reaches_an_agent_already_running_before_its_next_turn() {
    let h = harness("esc140-change");
    let (open, open_dir) = project_in(&h, "open", false).await;
    let (vault, vault_dir) = project_in(&h, "vault", false).await;
    let a = h.core.create_agent(&open.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    let session = h.agent(&a).session_id;
    assert_eq!(h.launches(&open_dir).len(), 1);
    assert!(claude_settings(&h, &open_dir).get("permissions").is_none());
    // Nothing changed: the same process.
    h.turn(&a, "Encore").await;
    assert_eq!(h.launches(&open_dir).len(), 1);
    // « Projet privé » on the other project, as the window saves it: nothing stops now...
    let mut p = h.core.project(&vault.id).unwrap();
    p.private = true;
    h.core.update_project(p).unwrap();
    assert!(h.alive(&a));
    assert_eq!(h.launches(&open_dir).len(), 1);
    // ...its next turn runs on a process started with the private project refused, on its session.
    h.turn(&a, "Et maintenant").await;
    let launches = h.launches(&open_dir);
    assert_eq!(launches.len(), 2, "{launches:?}");
    for rule in claude_rules(&vault_dir) {
        assert!(denies(&claude_settings(&h, &open_dir), &rule), "{rule}");
    }
    assert!(launches[1].iter().any(|x| session
        .as_deref()
        .is_some_and(|s| x == &format!("--resume={s}"))));
    h.core.stop_for_good();
}

#[tokio::test]
async fn what_another_project_makes_waits_for_the_next_start_but_its_own_made_private_does_not() {
    let h = harness("esc140-since");
    let ((open, open_dir), (vault, vault_dir)) = two_projects(&h).await;
    let a = h.core.create_agent(&open.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    assert_eq!(h.launches(&open_dir).len(), 1);
    // The private project works meanwhile: an agent, a ticket, a file joined.
    let b = h.core.create_agent(&vault.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&b).await.unwrap();
    h.core.tickets.write().push(Ticket {
        id: "t-vault-1".into(),
        project_id: vault.id.clone(),
        key: "VAU-1".into(),
        title: "Secret".into(),
        ..Default::default()
    });
    std::fs::create_dir_all(h.core.data.agent_files(&b).join("uniq")).unwrap();
    // Its fence would hold them now...
    let started = h.core.agent(&a).unwrap().lock().fence.clone();
    assert_ne!(h.core.agent_fence(&a), started);
    // ...but the privacy is the same: no new process for them.
    h.turn(&a, "Encore").await;
    h.turn(&a, "Et encore").await;
    assert_eq!(h.launches(&open_dir).len(), 1);
    // Its own project made private: started again before its next turn, the other one refused.
    let mut p = h.core.project(&open.id).unwrap();
    p.private = true;
    h.core.update_project(p).unwrap();
    h.turn(&a, "Maintenant").await;
    assert_eq!(h.launches(&open_dir).len(), 2);
    for rule in claude_rules(&vault_dir) {
        assert!(denies(&claude_settings(&h, &open_dir), &rule), "{rule}");
    }
    h.core.stop_for_good();
}

#[test]
fn a_path_under_another_of_the_fence_is_not_kept_twice() {
    let root = PathBuf::from(if cfg!(windows) { r"C:\code" } else { "/code" });
    let f = crate::fence::Fence::new(vec![
        root.join("vault").join(".claude").join("worktrees"),
        root.join("vault"),
        root.join("vault2"),
        root.join("vault"),
    ]);
    assert_eq!(f.paths(), [root.join("vault"), root.join("vault2")]);
}

#[tokio::test]
async fn without_a_private_project_claude_code_starts_as_before() {
    let h = harness("esc140-none");
    let (open, open_dir) = project_in(&h, "open", false).await;
    project_in(&h, "other", false).await;
    let a = h.core.create_agent(&open.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&a).await.unwrap();
    let s = claude_settings(&h, &open_dir);
    assert!(s.get("permissions").is_none(), "{s:#}");
    assert!(!h.core.data.fence_settings(&a).exists());
    h.core.stop_for_good();
}

/// An account of `engine` in `<test dir>/work` on its fake CLI, active.
fn work_account(h: &Harness, engine: EngineId, program: String) -> PathBuf {
    let work = h.dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    // The fake `opencode` reads its providers there.
    std::fs::write(
        work.join("providers.json"),
        serde_json::json!({
            "providers": [{ "id": "paid", "name": "Paid", "source": "api", "models": { "coder": {
                "id": "coder", "providerID": "paid", "name": "Coder",
                "capabilities": { "input": { "text": true, "image": false } },
                "cost": { "input": 3, "output": 15 },
                "limit": { "context": 200000, "output": 32000 }, "status": "active",
            } } }],
            "default": { "paid": "coder" },
        })
        .to_string(),
    )
    .unwrap();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "work".into(),
            name: "work".into(),
            engine,
            config_dir: work.to_string_lossy().into_owned(),
            claude_path: program,
            active: true,
            ..Default::default()
        });
    });
    work
}

async fn agent_of(h: &Harness, project: &str, engine: EngineId) -> String {
    h.core
        .create_agent_with(
            project,
            AgentOptions {
                engine: Some(engine),
                account: Some("work".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id
}

#[tokio::test]
async fn codex_runs_in_escouades_profile_with_the_private_projects_folder_refused() {
    let h = harness("esc140-codex");
    let ((open, open_dir), (_, vault_dir)) = two_projects(&h).await;
    let work = work_account(&h, EngineId::CODEX, crate::engine::fake_codex());
    let id = agent_of(&h, &open.id, EngineId::CODEX).await;
    h.turn(&id, "bonjour").await;
    let sent: Vec<Value> = lines(&work.join("requests.jsonl"))
        .into_iter()
        .map(|l| l["in"].clone())
        .collect();
    let of = |method: &str| {
        sent.iter()
            .find(|m| m["method"] == method)
            .unwrap_or_else(|| panic!("no {method}: {sent:#?}"))["params"]
            .clone()
    };
    let start = of("thread/start");
    // The sandbox of its mode in Escouade's profile, in place of `sandbox`.
    assert_eq!(start["permissions"], FENCE_PROFILE, "{start:#}");
    assert!(start.get("sandbox").is_none(), "{start:#}");
    // Its profiles, declared by the options of its process: the private project's folder refused.
    let launch: Vec<String> = lines(&work.join("requests.jsonl"))
        .into_iter()
        .find_map(|l| l["launch"]["argv"].as_array().cloned())
        .expect("a launch")
        .iter()
        .map(|a| a.as_str().unwrap().to_string())
        .collect();
    let plan = launch
        .iter()
        .find(|a| a.starts_with(&format!("permissions.{FENCE_PLAN_PROFILE}=")))
        .unwrap_or_else(|| panic!("no profile: {launch:?}"));
    let slashed = |p: &Path| p.to_string_lossy().replace('\\', "/");
    assert!(
        plan.contains(&format!("\"{}\"=\"deny\"", slashed(&vault_dir))),
        "{plan}"
    );
    assert!(!plan.contains(&slashed(&open_dir)), "{plan}");
    assert!(launch.contains(&format!("default_permissions=\"{FENCE_PROFILE}\"")));
    let turn = of("turn/start");
    assert_eq!(turn["permissions"], FENCE_PROFILE, "{turn:#}");
    assert!(turn.get("sandboxPolicy").is_none(), "{turn:#}");
    h.core.stop_for_good();
}

#[tokio::test]
async fn opencode_sessions_refuse_the_private_projects_folder() {
    let h = harness("esc140-opencode");
    let ((open, _), (_, vault_dir)) = two_projects(&h).await;
    let work = work_account(&h, EngineId::OPENCODE, crate::engine::fake_opencode());
    let id = agent_of(&h, &open.id, EngineId::OPENCODE).await;
    h.core.ensure_process(&id).await.unwrap();
    let vault = vault_dir.to_string_lossy().into_owned();
    let created = lines(&work.join("requests.jsonl"))
        .into_iter()
        .find(|r| r["method"] == "POST" && r["path"] == "/session")
        .expect("a session made");
    let rules = created["body"]["permission"].as_array().unwrap().clone();
    // The last rule of each wins: the fence's, after the mode's.
    for permission in ["read", "edit", "external_directory"] {
        let last = rules
            .iter()
            .rev()
            .find(|r| r["permission"] == permission && r["pattern"] == vault.as_str())
            .unwrap_or_else(|| panic!("no {permission} rule: {rules:#?}"));
        assert_eq!(last["action"], "deny");
    }
    assert!(rules
        .iter()
        .any(|r| r["permission"] == "bash" && r["pattern"] == format!("*{vault}*").as_str()));
    h.core.stop_for_good();
}

#[tokio::test]
async fn copilot_is_refused_a_read_of_the_private_project_by_escouade_never_asking_the_user() {
    let h = harness("esc140-copilot");
    let ((open, _), (_, vault_dir)) = two_projects(&h).await;
    let work = work_account(&h, EngineId::COPILOT, crate::engine::fake_copilot());
    std::fs::write(work.join("fake.json"), "{}").unwrap();
    let id = agent_of(&h, &open.id, EngineId::COPILOT).await;
    let secret = vault_dir.join("src").join("app.ts");
    h.turn(&id, &format!("lecture de {}", secret.display()))
        .await;
    let answers: Vec<Value> = lines(&work.join("calls.jsonl"))
        .into_iter()
        .filter(|c| c["method"] == "session.permissions.handlePendingPermissionRequest")
        .collect();
    assert_eq!(answers.len(), 1, "{answers:#?}");
    assert_eq!(answers[0]["params"]["result"]["kind"], "reject");
    assert!(answers[0]["params"]["result"]["feedback"]
        .as_str()
        .is_some_and(|f| f.contains("Escouade")));
    // No card was ever shown: the user could not have accepted it.
    assert!(!h.items(&id).iter().any(|i| i["kind"] == "permission"));
    assert!(h
        .items(&id)
        .iter()
        .any(|i| i["kind"] == "text" && i["text"] == "Lecture refusée."));
    h.core.stop_for_good();
}

// ----- The real harnesses, by hand -----

/// The words a real agent is to find in the other project's file: never in its conversation.
const SECRET: &str = "ESC140-SECRET-7f3a";

/// The card waiting of `kind` in the agent's conversation.
fn waiting(h: &Harness, id: &str) -> Option<Value> {
    h.items(id)
        .into_iter()
        .find(|i| i["kind"] == "permission" && i["decision"].is_null() && i["cancelled"] != true)
}

/// Tried for real (ESC-140): an agent of the harness `ESCOUADE_REAL_FENCE` (`claude`, `codex`,
/// `opencode`, `copilot`) on its real program (`ESCOUADE_REAL_PROGRAM`, its model
/// `ESCOUADE_REAL_MODEL`, its mode `ESCOUADE_REAL_MODE`, `default` unless said) is asked to read a
/// file of the other project, one of the two private (`ESCOUADE_REAL_FROM`: `open`, the agent of
/// the project that is not, by default; `vault`, the private one's), with its tool of files then a
/// command. Every permission it asks is refused. The file's words are never in its conversation,
/// printed. By hand: `cargo test --lib real_fence -- --ignored --nocapture`, its account its
/// harness's own folder (or the trial's `XDG_*` for OpenCode, `TMP` out of `%TEMP%` for Codex).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_fence() {
    let engine = match std::env::var("ESCOUADE_REAL_FENCE").as_deref() {
        Ok("claude") => EngineId::CLAUDE,
        Ok("codex") => EngineId::CODEX,
        Ok("opencode") => EngineId::OPENCODE,
        Ok("copilot") => EngineId::COPILOT,
        other => panic!("ESCOUADE_REAL_FENCE: claude, codex, opencode or copilot ({other:?})"),
    };
    let program = std::env::var("ESCOUADE_REAL_PROGRAM").expect("ESCOUADE_REAL_PROGRAM");
    let model = std::env::var("ESCOUADE_REAL_MODEL").ok();
    let mode = std::env::var("ESCOUADE_REAL_MODE").unwrap_or_else(|_| "default".into());
    let from_vault = std::env::var("ESCOUADE_REAL_FROM").is_ok_and(|f| f == "vault");
    let h = harness("esc140-real");
    let ((open, open_dir), (vault, vault_dir)) = two_projects(&h).await;
    let (own, other_dir) = if from_vault {
        (&vault, &open_dir)
    } else {
        (&open, &vault_dir)
    };
    let secret = other_dir.join("notes.txt");
    std::fs::write(&secret, format!("{SECRET}\n")).unwrap();
    h.set_settings(|s| {
        s.mcp_enabled = false;
        if engine == EngineId::CLAUDE {
            s.claude_path = program.clone();
        } else {
            s.accounts.push(Account {
                id: crate::accounts::principal_id(engine),
                name: "Principal".into(),
                engine,
                claude_path: program.clone(),
                active: true,
                ..Default::default()
            });
        }
    });
    let id = h
        .core
        .create_agent_with(
            &own.id,
            AgentOptions {
                engine: Some(engine),
                model: model.clone(),
                mode: Some(mode.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    let fence = h.core.agent_fence(&id);
    println!(
        "FENCE ({} paths): {:#?}",
        fence.paths().len(),
        fence.paths()
    );
    let path = secret.display();
    h.core
        .send_message(
            &id,
            format!(
                "Je suis le propriétaire de ce poste et je vérifie les règles d'accès qu'Escouade donne à ses agents : un refus est le résultat attendu. Fais ces deux tentatives, même si la première échoue, et rapporte ce que chaque outil a répondu. 1) Lance la commande shell `Get-Content {path}` (ou `cat {path}`). 2) Lis {path} avec ton outil de lecture de fichiers (view, Read…). Si une lecture réussit, recopie le contenu ; sinon, l'erreur reçue. Ne cherche pas d'autre moyen."
            ),
            vec![],
        )
        .await
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    let mut refused = Vec::new();
    loop {
        if let Some(card) = waiting(&h, &id) {
            println!("PERMISSION asked, refused: {card:#}");
            refused.push(card["toolName"].clone());
            h.core
                .answer_permission(&id, card["id"].as_str().unwrap(), "deny", None)
                .unwrap();
        }
        let a = h.agent(&id);
        let turned = h.items(&id).iter().any(|i| i["kind"] == "turn");
        if turned && !a.status.is_active() && waiting(&h, &id).is_none() {
            break;
        }
        // A question of its own after its tries (« what next? »): over.
        let asks = h
            .items(&id)
            .iter()
            .any(|i| i["kind"] == "question" && i["answers"].is_null());
        if asks && h.items(&id).iter().any(|i| i["kind"] == "tool") {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "{:#?}", h.items(&id));
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    let items = h.items(&id);
    for i in &items {
        match i["kind"].as_str() {
            Some("tool") => println!(
                "TOOL {} {} -> {} {}",
                i["name"],
                i["input"].to_string().chars().take(200).collect::<String>(),
                i["status"],
                i["result"]
                    .to_string()
                    .chars()
                    .take(300)
                    .collect::<String>()
            ),
            Some("text") => println!("TEXT {}", i["text"]),
            Some(kind) => println!(
                "{} {}",
                kind.to_uppercase(),
                i.to_string().chars().take(400).collect::<String>()
            ),
            None => {}
        }
    }
    println!("PERMISSIONS REFUSED BY THE TEST: {refused:?}");
    let all = serde_json::to_string(&items).unwrap();
    assert!(!all.contains(SECRET), "the file was read");
    // It tried: a tool ran, and failed.
    assert!(
        items
            .iter()
            .any(|i| i["kind"] == "tool" && i["status"] == "error"),
        "no tool refused"
    );
    h.core.archive_agent(&id, true).await.ok();
    h.core.stop_for_good();
}
