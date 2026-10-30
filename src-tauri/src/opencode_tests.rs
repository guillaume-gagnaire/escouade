//! OpenCode beside Claude Code, through the core (ESC-44): an agent of it converses, asks its
//! permissions and its questions, keeps a message sent during a turn for after it, fills the
//! band « Plan » and lists the models of the connected providers. Its agents run the fake
//! `opencode serve` (tests/fixtures/fake-opencode.mjs).

use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::engine::{registry, Program, OPENCODE};
use crate::model::*;
use serde_json::{json, Value};

/// OpenCode's Principal, on the fake `opencode`.
fn opencode_account(h: &Harness) {
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-opencode".into(),
            name: "Principal".into(),
            engine: EngineId::OPENCODE,
            config_dir: String::new(),
            claude_path: crate::engine::fake_opencode(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            ..Default::default()
        });
    });
}

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

async fn new_agent(h: &Harness, project: &str) -> String {
    let id = h
        .core
        .create_agent_with(
            project,
            AgentOptions {
                engine: Some(EngineId::OPENCODE),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.wait("its session and the models", |h| {
        h.agent(id.as_str()).session_id.is_some()
            && h.core
                .engine_models
                .read()
                .get(&EngineId::OPENCODE)
                .is_some_and(|m| !m.is_empty())
    })
    .await;
    id
}

#[tokio::test]
async fn an_agent_of_opencode_converses_asks_its_permissions_and_its_questions() {
    let h = harness("esc44-converse");
    let (p, _) = h.project(false).await;
    opencode_account(&h);
    let id = new_agent(&h, &p.id).await;
    // A model of the connected providers, never one OpenCode would pick itself.
    assert_eq!(h.agent(&id).model, "ollama/qwen3-coder:30b-32k");
    assert_eq!(h.agent(&id).engine, EngineId::OPENCODE);
    h.turn(&id, "bonjour").await;
    assert!(texts(&h, &id).contains(&"Bonjour, tu as dit : bonjour".to_string()));
    let turn = h
        .items(&id)
        .into_iter()
        .rev()
        .find(|i| i["kind"] == "turn")
        .unwrap();
    assert_eq!(turn["engine"], "opencode");
    assert_eq!(turn["tokens"], 130);

    // A command its rules ask for: its card, allowed, and a message sent meanwhile kept for after.
    h.core
        .send_message(&id, "commande".into(), vec![])
        .await
        .unwrap();
    h.wait("the permission", |h| {
        waiting(h, &id, "permission").is_some()
    })
    .await;
    let card = waiting(&h, &id, "permission").unwrap();
    assert_eq!(card["toolName"], "Bash");
    assert_eq!(card["input"]["command"], "rm -rf build");
    assert!(card["description"].as_str().unwrap().contains("rm *"));
    assert_eq!(h.agent(&id).status, AgentStatus::Waiting);
    h.core
        .send_message(&id, "et ensuite".into(), vec![])
        .await
        .unwrap();
    let queued = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "user" && i["text"] == "et ensuite")
        .unwrap();
    assert_eq!(queued["queued"], true);
    h.core
        .answer_permission(&id, card["id"].as_str().unwrap(), "allow", None)
        .unwrap();
    h.wait("the message kept, answered", |h| {
        texts(h, &id).contains(&"Bonjour, tu as dit : et ensuite".to_string())
            && !h.agent(&id).status.is_active()
    })
    .await;
    let bash = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "tool" && i["name"] == "Bash")
        .unwrap();
    assert_eq!(bash["status"], "ok");
    assert!(texts(&h, &id).contains(&"La commande est passée.".to_string()));

    // Refused with a word: the agent is told it.
    h.core
        .send_message(&id, "commande".into(), vec![])
        .await
        .unwrap();
    h.wait("the permission", |h| {
        waiting(h, &id, "permission").is_some()
    })
    .await;
    let card = waiting(&h, &id, "permission").unwrap();
    h.core
        .answer_permission(
            &id,
            card["id"].as_str().unwrap(),
            "deny",
            Some("pas maintenant".into()),
        )
        .unwrap();
    h.wait("the refusal said back", |h| {
        texts(h, &id).iter().any(|t| t.contains("pas maintenant"))
    })
    .await;

    // A question: its card, answered.
    h.wait("the turn over", |h| !h.agent(&id).status.is_active())
        .await;
    h.core
        .send_message(&id, "question".into(), vec![])
        .await
        .unwrap();
    h.wait("the question", |h| waiting(h, &id, "question").is_some())
        .await;
    let card = waiting(&h, &id, "question").unwrap();
    assert_eq!(card["questions"][0]["options"][1]["label"], "Rust");
    h.core
        .answer_question(
            &id,
            card["id"].as_str().unwrap(),
            json!({ "Quel langage préfères-tu ?": "Rust" }),
        )
        .unwrap();
    h.wait("the answer said back", |h| {
        texts(h, &id).contains(&"Tu as choisi : Rust".to_string())
    })
    .await;
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_tasks_and_the_subagents_of_opencode_fill_the_plan() {
    let h = harness("esc44-plan");
    let (p, _) = h.project(false).await;
    opencode_account(&h);
    let id = new_agent(&h, &p.id).await;
    h.turn(&id, "étapes").await;
    let plan = h.agent(&id).plan.expect("its plan");
    assert_eq!(plan.tasks.len(), 3);
    assert!(plan
        .tasks
        .iter()
        .all(|t| t.status == crate::plan::TaskStatus::Done));
    h.turn(&id, "sous-agent").await;
    let plan = h.agent(&id).plan.expect("its plan");
    assert_eq!(plan.agents.len(), 1, "{plan:?}");
    let row = &plan.agents[0];
    assert_eq!(row.title, "Explorer le dépôt");
    assert_eq!(row.status, crate::plan::RunStatus::Done);
    assert_eq!(row.tools, 1);
    assert_eq!(row.tokens, Some(60));
    let child = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "tool" && i["name"] == "Glob")
        .unwrap();
    assert_eq!(child["parent"], row.id.as_str());
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_models_of_the_connected_providers_come_with_their_variants_and_favorites() {
    let h = harness("esc44-models");
    let (p, _) = h.project(false).await;
    opencode_account(&h);
    let id = new_agent(&h, &p.id).await;
    // Listed first by `opencode models` (the providers by their ids), then by its server (their
    // names), which may come after the agent is there.
    h.wait("the models of its server", |h| {
        h.core.engine_models.read()[&EngineId::OPENCODE]
            .first()
            .is_some_and(|m| m.provider.as_deref() == Some("Ollama (local)"))
    })
    .await;
    let models = h.core.engine_models.read()[&EngineId::OPENCODE].clone();
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "ollama/gpt-oss:20b-32k",
            "ollama/qwen3-coder:30b-32k",
            "lmstudio/devstral-small"
        ]
    );
    let gpt = &models[0];
    assert_eq!(gpt.efforts, ["low", "medium", "high"]);
    assert_eq!(gpt.provider.as_deref(), Some("Ollama (local)"));
    // The effort of a model that has it is its variant, at the next prompt.
    h.core
        .set_agent_options(
            &id,
            Some("ollama/gpt-oss:20b-32k".into()),
            Some("high".into()),
            None,
        )
        .await
        .unwrap();
    h.turn(&id, "bonjour").await;
    assert!(texts(&h, &id).contains(&"Bonjour, tu as dit : bonjour (variante high)".to_string()));
    // A model without variants is sent none.
    h.core
        .set_agent_options(&id, Some("ollama/qwen3-coder:30b-32k".into()), None, None)
        .await
        .unwrap();
    h.turn(&id, "salut").await;
    assert!(texts(&h, &id).contains(&"Bonjour, tu as dit : salut".to_string()));
    // Favorites, kept at once.
    let favs = h
        .core
        .set_favorite_model(EngineId::OPENCODE, "lmstudio/devstral-small", true)
        .unwrap();
    assert_eq!(favs[&EngineId::OPENCODE], ["lmstudio/devstral-small"]);
    h.core
        .set_favorite_model(EngineId::OPENCODE, "ollama/gpt-oss:20b-32k", true)
        .unwrap();
    let favs = h
        .core
        .set_favorite_model(EngineId::OPENCODE, "lmstudio/devstral-small", false)
        .unwrap();
    assert_eq!(favs[&EngineId::OPENCODE], ["ollama/gpt-oss:20b-32k"]);
    assert_eq!(
        h.core.settings.read().favorite_models[&EngineId::OPENCODE],
        ["ollama/gpt-oss:20b-32k"]
    );
    h.core.stop_for_good();
}

#[tokio::test]
async fn opencode_2_is_recognized_and_refused_with_its_reason() {
    let dir = crate::paths::test_dir("esc44-v2");
    let (file, body) = if cfg!(windows) {
        ("opencode.cmd", "@echo opencode v2.0.32\r\n")
    } else {
        ("opencode", "#!/bin/sh\necho 'opencode v2.0.32'\n")
    };
    let path = dir.join(file);
    std::fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let program = Program::Native(path);
    let e = registry::check(&OPENCODE, &program).await.unwrap_err();
    let said = format!("{e:#}");
    assert!(
        said.contains("2.0.32") && said.contains("opencode-ai"),
        "{said}"
    );
    let version = registry::version(&OPENCODE, &program).await;
    let (state, note) = registry::state_of(
        crate::i18n::Lang::En,
        &OPENCODE,
        true,
        version.as_deref(),
        true,
    );
    assert_eq!(state, registry::EngineState::Unsupported);
    assert!(note.unwrap().contains("line 2"));
    // OpenCode 1 is taken.
    let (state, _) =
        registry::state_of(crate::i18n::Lang::En, &OPENCODE, true, Some("1.19.3"), true);
    assert_eq!(state, registry::EngineState::Ready);
}

/// Waits, up to `secs`, for what a real model takes its time with.
async fn wait_long(h: &Harness, what: &str, secs: u64, pred: impl Fn(&Harness) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        if pred(h) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    let items = h.items(
        &h.core
            .agents
            .read()
            .keys()
            .next()
            .cloned()
            .unwrap_or_default(),
    );
    panic!("timed out waiting for {what}: {items:#?}");
}

/// Real run (ESC-44): an agent on the real OpenCode 1 that `ESCOUADE_REAL_OPENCODE` names, with a
/// local model of Ollama (`qwen3-coder:30b-32k`). By hand only, its OpenCode folders kept out of
/// the user's by `XDG_*` (a folder whose `config/opencode/opencode.json` declares the provider
/// `ollama`): `cargo test --lib real_opencode -- --ignored --nocapture` (docs/TODO-1.8.md, point 0,
/// « OpenCode »). It converses, asks a permission (allowed), a question (answered), keeps a task
/// list and runs a subagent; it prints the conversation.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_opencode() {
    let Ok(real) = std::env::var("ESCOUADE_REAL_OPENCODE") else {
        panic!("ESCOUADE_REAL_OPENCODE: the path of the real opencode");
    };
    assert!(
        std::env::var("XDG_CONFIG_HOME").is_ok(),
        "XDG_CONFIG_HOME and the others: OpenCode's folders out of the user's"
    );
    let h = harness("esc44-real");
    let (p, _) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-opencode".into(),
            name: "Principal".into(),
            engine: EngineId::OPENCODE,
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
                engine: Some(EngineId::OPENCODE),
                model: Some("ollama/qwen3-coder:30b-32k".into()),
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
    wait_long(&h, "the first reply", 600, |h| ended(h, 1)).await;
    assert!(texts(&h, &id).iter().any(|t| t.contains('A')));
    assert_eq!(h.agent(&id).model, "ollama/qwen3-coder:30b-32k");

    send("Avec l'outil bash, lance la commande `git status --short`, puis dis-moi ce qu'elle a affiché.").await;
    wait_long(&h, "the permission", 600, |h| {
        waiting(h, &id, "permission").is_some()
    })
    .await;
    let card = waiting(&h, &id, "permission").unwrap();
    println!("PERMISSION {card:#}");
    assert_eq!(card["toolName"], "Bash");
    h.core
        .answer_permission(&id, card["id"].as_str().unwrap(), "allow", None)
        .unwrap();
    wait_long(&h, "the command's turn", 600, |h| ended(h, 2)).await;
    assert!(h
        .items(&id)
        .iter()
        .any(|i| i["kind"] == "tool" && i["name"] == "Bash" && i["status"] == "ok"));

    send("Pose-moi une question avec l'outil question : « Quelle couleur ? », avec les options « rouge » et « bleu ». Puis réponds seulement par la couleur choisie.").await;
    wait_long(&h, "the question", 600, |h| {
        waiting(h, &id, "question").is_some()
    })
    .await;
    let card = waiting(&h, &id, "question").unwrap();
    println!("QUESTION {card:#}");
    let q = card["questions"][0]["question"]
        .as_str()
        .unwrap()
        .to_string();
    let bleu = card["questions"][0]["options"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["label"].as_str())
        .find(|l| l.to_lowercase().contains("bleu"))
        .unwrap_or("bleu")
        .to_string();
    h.core
        .answer_question(&id, card["id"].as_str().unwrap(), json!({ q: bleu }))
        .unwrap();
    wait_long(&h, "the question's turn", 600, |h| ended(h, 3)).await;
    assert!(texts(&h, &id)
        .last()
        .is_some_and(|t| t.to_lowercase().contains("bleu")));

    send("Écris avec l'outil todowrite une liste de deux tâches (« Lire calc », « Relire calc »), puis marque-les toutes les deux terminées avec todowrite. Ne fais rien d'autre.").await;
    wait_long(&h, "the task list's turn", 600, |h| ended(h, 4)).await;
    let plan = h.agent(&id).plan.expect("its tasks");
    println!("TASKS {:?}", plan.tasks);
    assert!(!plan.tasks.is_empty());

    send("Utilise l'outil task avec le sous-agent « explore » pour lister les fichiers du dépôt. Ne fais rien d'autre toi-même.").await;
    // Its subagent's commands ask too (the rules of the server's config): each is allowed.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(900);
    let mut asked_by_sub = 0;
    while !ended(&h, 5) && std::time::Instant::now() < deadline {
        if let Some(card) = waiting(&h, &id, "permission") {
            println!("SUB PERMISSION {card:#}");
            if card["sub"].is_string() {
                asked_by_sub += 1;
            }
            h.core
                .answer_permission(&id, card["id"].as_str().unwrap(), "allow", None)
                .unwrap();
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    wait_long(&h, "the subagent's turn", 60, |h| ended(h, 5)).await;
    println!("PERMISSIONS ASKED BY THE SUBAGENT: {asked_by_sub}");
    let plan = h.agent(&id).plan.expect("its subagent");
    println!("SUBAGENTS {:?}", plan.agents);
    assert!(plan
        .agents
        .iter()
        .any(|a| a.status == crate::plan::RunStatus::Done));

    for i in h.items(&id) {
        let text = i["text"].as_str().unwrap_or_default();
        println!(
            "{} {} {} {}",
            i["kind"],
            i["name"].as_str().or(i["toolName"].as_str()).unwrap_or(""),
            i["status"].as_str().unwrap_or(""),
            crate::text::truncate(text, 120)
        );
    }
    h.core.stop_for_good();
}

/// What the fake `opencode` noted of its launches in the folder of an account
/// (`<OPENCODE_CONFIG_DIR>/launches.jsonl`).
fn launches(dir: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("launches.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn no_server_starts_without_a_model_of_its_providers_and_a_server_always_has_its_small_model()
{
    let h = harness("esc44-no-model");
    let (p, _) = h.project(false).await;
    let dir = h.dir.join("oc-work");
    std::fs::create_dir_all(&dir).unwrap();
    // No provider connected to OpenCode: nothing to run on.
    std::fs::write(
        dir.join("providers.json"),
        r#"{ "providers": [], "default": {} }"#,
    )
    .unwrap();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "oc-work".into(),
            name: "Travail".into(),
            engine: EngineId::OPENCODE,
            config_dir: dir.to_string_lossy().into_owned(),
            claude_path: crate::engine::fake_opencode(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            ..Default::default()
        });
    });
    // A model of another harness (a handover, a setting of Claude Code's).
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::OPENCODE),
                model: Some("sonnet".into()),
                account: Some("oc-work".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    let e = h
        .core
        .send_message(&id, "bonjour".into(), vec![])
        .await
        .err()
        .map(|e| format!("{e:#}"))
        .unwrap_or_default();
    let notices: Vec<String> = h
        .items(&id)
        .iter()
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect();
    assert!(
        e.contains("Aucun modèle d’OpenCode")
            || notices
                .iter()
                .any(|t| t.contains("Aucun modèle d’OpenCode")),
        "{e} / {notices:?}"
    );
    // Its models were asked (`opencode models`), and no server started.
    let asked = launches(&dir);
    assert!(asked.iter().all(|l| l["argv"][0] != "serve"), "{asked:?}");
    assert!(asked.iter().any(|l| l["argv"][0] == "models"), "{asked:?}");
    assert_eq!(h.agent(&id).model, "sonnet");

    // A provider connected: the agent takes its default model, and the server it starts has it as
    // its `model` and its `small_model`.
    std::fs::remove_file(dir.join("providers.json")).unwrap();
    h.turn(&id, "bonjour").await;
    assert_eq!(h.agent(&id).model, "ollama/qwen3-coder:30b-32k");
    let serve = launches(&dir)
        .into_iter()
        .find(|l| l["argv"][0] == "serve")
        .expect("its server");
    assert_eq!(serve["config"]["model"], "ollama/qwen3-coder:30b-32k");
    assert_eq!(serve["config"]["small_model"], "ollama/qwen3-coder:30b-32k");
    h.core.stop_for_good();
}

/// Real run (ESC-44): the models of the real OpenCode (`ESCOUADE_REAL_OPENCODE`, its folders by
/// `XDG_*` as for `real_opencode`), listed without a server. By hand only.
#[tokio::test]
#[ignore]
async fn real_opencode_models() {
    use crate::engine::Engine;
    let real = std::env::var("ESCOUADE_REAL_OPENCODE").expect("ESCOUADE_REAL_OPENCODE");
    let program = Program::Native(real.into());
    let models = OPENCODE
        .list_models(program, Vec::new(), &std::env::temp_dir())
        .await
        .unwrap()
        .expect("its models");
    println!("{models:#?}");
    assert!(models.iter().any(|m| m.id == "ollama/qwen3-coder:30b-32k"));
    assert_eq!(models.iter().filter(|m| m.default).count(), 1);
}

/// What the terminal `screen` shows, as text, until it has `count` times `what` (60 s at most);
/// the pseudo-console's cursor query answered on the way, as xterm.js does.
async fn wait_screen(
    h: &Harness,
    term: &str,
    screen: &std::sync::Arc<parking_lot::Mutex<String>>,
    what: &str,
    count: usize,
) {
    let start = std::time::Instant::now();
    let mut answered = false;
    while crate::core_tests::plain_text(&screen.lock())
        .matches(what)
        .count()
        < count
    {
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(term, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(60),
            "{what} ({count}) not in {:?}",
            crate::core_tests::plain_text(&screen.lock())
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

/// « Vue OpenCode » (ESC-46): `opencode attach` joins the agent's session on its account's server,
/// its password in its environment only, while Escouade's own client goes on: the conversation
/// keeps up with what the interface does, a permission answered on one side is seen on the other,
/// and leaving the view stops the interface alone.
#[tokio::test]
async fn the_opencode_view_joins_the_agents_session_beside_escouade() {
    let h = harness("esc46-attach");
    let (p, _) = h.project(false).await;
    let dir = h.dir.join("oc-view");
    std::fs::create_dir_all(&dir).unwrap();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "oc-view".into(),
            name: "Vue".into(),
            engine: EngineId::OPENCODE,
            config_dir: dir.to_string_lossy().into_owned(),
            claude_path: crate::engine::fake_opencode(),
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
                engine: Some(EngineId::OPENCODE),
                account: Some("oc-view".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.wait("its session", |h| h.agent(id.as_str()).session_id.is_some())
        .await;
    let session = h.agent(&id).session_id.unwrap();
    let cwd = h.agent(&id).cwd;

    h.core.set_agent_view(&id, true).await.unwrap();
    let m = h.agent(&id);
    assert!(m.attached && !m.terminal, "{m:?}");
    let screen = std::sync::Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(&id, (100, 30), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    wait_screen(&h, &info.id, &screen, &format!("({session})"), 1).await;
    // On the agent's server and session, in its folder; the password in its environment alone.
    let attaches: Vec<Value> = std::fs::read_to_string(dir.join("attaches.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(attaches.len(), 1);
    let argv: Vec<&str> = attaches[0]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(argv[0], "attach");
    assert!(argv[1].starts_with("http://127.0.0.1:"), "{argv:?}");
    assert_eq!(&argv[2..], ["--session", &session, "--dir", &cwd]);
    let password = attaches[0]["password"].as_str().unwrap();
    assert_eq!(password.len(), 48);
    assert!(!argv.iter().any(|a| a.contains(password)));
    // Nor on the server's command line, nor in the app's log of the launch.
    for l in launches(&dir) {
        assert!(!l["argv"].to_string().contains(password));
    }
    // The agent's terminal is the interface's, its process still Escouade's.
    assert_eq!(
        h.core.agent(&id).unwrap().lock().view().terminal_id,
        Some(info.id.clone())
    );
    assert!(h.core.agent(&id).unwrap().lock().view().alive);

    // A message of Escouade's: the interface sees its turn; its permission answered there...
    h.core
        .send_message(&id, "commande".into(), vec![])
        .await
        .unwrap();
    h.wait("the permission", |h| {
        waiting(h, &id, "permission").is_some()
    })
    .await;
    let card = waiting(&h, &id, "permission").unwrap();
    wait_screen(&h, &info.id, &screen, "permission bash: rm -rf build", 1).await;
    h.core.pty.write(&info.id, b"\r").unwrap();
    // ...is seen by Escouade: the card says so, and the turn goes on to its end.
    h.wait("the card answered in the interface", |h| {
        h.items(&id)
            .iter()
            .any(|i| i["id"] == card["id"] && i["decision"] == "allow")
    })
    .await;
    h.wait("the end of the turn", |h| {
        texts(h, &id).contains(&"La commande est passée.".to_string())
            && !h.agent(&id).status.is_active()
    })
    .await;
    wait_screen(&h, &info.id, &screen, "text: La commande est passée.", 1).await;

    // Answered in Escouade: the interface sees it.
    h.core
        .send_message(&id, "commande".into(), vec![])
        .await
        .unwrap();
    h.wait("the second permission", |h| {
        waiting(h, &id, "permission").is_some()
    })
    .await;
    wait_screen(&h, &info.id, &screen, "permission bash: rm -rf build", 2).await;
    let card = waiting(&h, &id, "permission").unwrap();
    h.core
        .answer_permission(&id, card["id"].as_str().unwrap(), "allow", None)
        .unwrap();
    wait_screen(&h, &info.id, &screen, "permission answered: once", 2).await;
    h.wait("the end of the second turn", |h| {
        !h.agent(&id).status.is_active()
    })
    .await;

    // A message typed in the interface: Escouade's conversation has its turn.
    h.core.pty.write(&info.id, b"depuis l'interface\r").unwrap();
    h.wait("the turn typed in the interface", |h| {
        texts(h, &id).contains(&"Bonjour, tu as dit : depuis l'interface".to_string())
            && !h.agent(&id).status.is_active()
    })
    .await;
    // The message typed there is in the conversation, once, before its reply; the two of
    // Escouade's, sent while the interface was open, once each too (no echo of the server's).
    let users: Vec<Value> = h
        .items(&id)
        .into_iter()
        .filter(|i| i["kind"] == "user")
        .collect();
    let said: Vec<&str> = users.iter().map(|u| u["text"].as_str().unwrap()).collect();
    assert_eq!(said, ["commande", "commande", "depuis l'interface"]);
    assert_eq!(users[2]["origin"], "terminal");
    let items = h.items(&id);
    let at = |pred: &dyn Fn(&Value) -> bool| items.iter().position(pred).unwrap();
    assert!(
        at(&|i| i["kind"] == "user" && i["text"] == "depuis l'interface")
            < at(&|i| i["text"] == "Bonjour, tu as dit : depuis l'interface")
    );

    // A window reloaded takes the same interface over.
    let again = h
        .core
        .open_agent_terminal(&id, (100, 30), false, |_| {})
        .await
        .unwrap();
    assert_eq!(again.id, info.id);

    // Its process started again on a new server (the account's only agent): the interface, which
    // talks to the old one, stops; the next one joins the new.
    h.core.restart_process(&id).await.unwrap();
    h.wait("the interface stopped", |h| !h.core.pty.exists(&info.id))
        .await;
    h.wait("the agent without it", |h| {
        h.core
            .agent(&id)
            .unwrap()
            .lock()
            .view()
            .terminal_id
            .is_none()
    })
    .await;
    assert!(h.agent(&id).attached);
    let screen = std::sync::Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(&id, (100, 30), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    wait_screen(&h, &info.id, &screen, &format!("({session})"), 1).await;

    // Leaving the view stops the interface alone: the process and its server go on.
    h.core.set_agent_view(&id, false).await.unwrap();
    assert!(!h.agent(&id).attached);
    assert!(!h.core.pty.exists(&info.id));
    assert_eq!(h.core.agent(&id).unwrap().lock().view().terminal_id, None);
    h.turn(&id, "encore").await;
    assert!(texts(&h, &id).contains(&"Bonjour, tu as dit : encore".to_string()));
    let serves = launches(&dir)
        .iter()
        .filter(|l| l["argv"][0] == "serve")
        .count();
    assert_eq!(serves, 2);
}

/// « Vue des nouveaux agents » on « Terminal du harness »: a new agent of OpenCode starts in its view,
/// joined (never `terminal`); a ticket's agent stays in Escouade's, and is refused it.
#[tokio::test]
async fn a_new_agent_of_opencode_starts_in_its_view_when_the_settings_say_so() {
    let h = harness("esc46-new-view");
    let (p, _) = h.project(false).await;
    opencode_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = new_agent(&h, &p.id).await;
    let m = h.agent(&id);
    assert!(m.attached && !m.terminal);
    let ticket = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::OPENCODE),
                ticket_id: Some("k1".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    assert!(!ticket.attached && !ticket.terminal);
    let e = h.core.set_agent_view(&ticket.id, true).await.unwrap_err();
    assert!(format!("{e:#}").contains("ticket"), "{e:#}");
    assert!(!h.agent(&ticket.id).attached);
}

/// Real run of « Vue OpenCode » (ESC-46): the real `opencode attach` (`ESCOUADE_REAL_OPENCODE`, its
/// folders kept out of the user's by `XDG_*`, as `real_opencode`) joined to an agent's session on
/// a local model of Ollama, beside Escouade. By hand only:
/// `cargo test --lib real_opencode_view -- --ignored --nocapture`. A permission of a message of
/// Escouade's is answered in the interface (Enter: « Allow once »), another in Escouade; a message
/// typed in the interface is a turn of the conversation; no process's command line has the
/// server's password; leaving the view leaves the server up.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_opencode_view() {
    let Ok(real) = std::env::var("ESCOUADE_REAL_OPENCODE") else {
        panic!("ESCOUADE_REAL_OPENCODE: the path of the real opencode");
    };
    assert!(
        std::env::var("XDG_CONFIG_HOME").is_ok(),
        "XDG_CONFIG_HOME and the others: OpenCode's folders out of the user's"
    );
    let h = harness("esc46-real");
    let (p, _) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-opencode".into(),
            name: "Principal".into(),
            engine: EngineId::OPENCODE,
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
                engine: Some(EngineId::OPENCODE),
                model: Some("ollama/qwen3-coder:30b-32k".into()),
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
    wait_long(&h, "the first reply", 600, |h| ended(h, 1)).await;

    h.core.set_agent_view(&id, true).await.unwrap();
    let screen = std::sync::Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(&id, (120, 40), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    // Its screen shows the session: the message sent from Escouade.
    wait_screen(&h, &info.id, &screen, "Réponds seulement", 1).await;
    let password = {
        let rt = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
        rt.attach().unwrap().env[0].1.clone()
    };
    let session = h.agent(&id).session_id.unwrap();
    let lines = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Process | ForEach-Object { $_.CommandLine }",
        ])
        .output()
        .unwrap();
    let lines = String::from_utf8_lossy(&lines.stdout);
    assert!(
        lines.contains(&format!("--session {session}")),
        "the interface's command line is listed"
    );
    assert!(
        !lines.contains(&password),
        "the password is on a command line"
    );

    // A permission of Escouade's message, answered in the interface.
    send("Avec l'outil bash, lance la commande `git status --short`, puis dis-moi ce qu'elle a affiché.").await;
    wait_long(&h, "the permission", 600, |h| {
        waiting(h, &id, "permission").is_some()
    })
    .await;
    let card = waiting(&h, &id, "permission").unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    h.core.pty.write(&info.id, b"\r").unwrap();
    wait_long(&h, "the card answered in the interface", 60, |h| {
        h.items(&id)
            .iter()
            .any(|i| i["id"] == card["id"] && i["decision"] == "allow")
    })
    .await;
    wait_long(&h, "the command's turn", 600, |h| ended(h, 2)).await;

    // Another, answered in Escouade: the interface goes on with the turn (the commit it shows).
    let head = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(&p.path)
        .output()
        .unwrap();
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    send("Avec l'outil bash, lance la commande `git log --oneline -1`, puis recopie sa sortie.")
        .await;
    wait_long(&h, "the second permission", 600, |h| {
        waiting(h, &id, "permission").is_some()
    })
    .await;
    let card = waiting(&h, &id, "permission").unwrap();
    h.core
        .answer_permission(&id, card["id"].as_str().unwrap(), "allow", None)
        .unwrap();
    wait_long(&h, "the second command's turn", 600, |h| ended(h, 3)).await;
    wait_screen(&h, &info.id, &screen, &head, 1).await;

    // A message typed in the interface: a turn of Escouade's conversation.
    let typed = crate::engine::terminal::bracketed("Réponds seulement : B");
    h.core.pty.write(&info.id, &typed).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    h.core.pty.write(&info.id, b"\r").unwrap();
    wait_long(&h, "the turn typed in the interface", 600, |h| ended(h, 4)).await;
    assert!(texts(&h, &id).last().is_some_and(|t| t.contains('B')));
    // The message itself too, once, as the user's.
    let typed = h
        .items(&id)
        .into_iter()
        .filter(|i| i["kind"] == "user" && i["text"] == "Réponds seulement : B")
        .count();
    assert_eq!(typed, 1);

    // Back in Escouade's view: the interface stops, the server goes on.
    h.core.set_agent_view(&id, false).await.unwrap();
    assert!(!h.core.pty.exists(&info.id));
    send("Réponds seulement : C").await;
    wait_long(&h, "the reply after the view", 600, |h| ended(h, 5)).await;
    assert!(texts(&h, &id).last().is_some_and(|t| t.contains('C')));
    for i in h.items(&id) {
        let text = i["text"].as_str().unwrap_or_default();
        println!(
            "{} {} {} {} {}",
            i["kind"],
            i["name"].as_str().or(i["toolName"].as_str()).unwrap_or(""),
            i["status"].as_str().unwrap_or(""),
            i["decision"].as_str().unwrap_or(""),
            crate::text::truncate(text, 120)
        );
    }
    h.core.stop_for_good();
}

/// An account of OpenCode in `h.dir/<id>`, whose providers are `providers` (as `/config/providers`
/// answers them), with `budget` as its « Budget par jour ($) ».
fn account_with(
    h: &Harness,
    id: &str,
    providers: Value,
    budget: Option<f64>,
) -> std::path::PathBuf {
    let dir = h.dir.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("providers.json"), providers.to_string()).unwrap();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: id.into(),
            name: id.into(),
            engine: EngineId::OPENCODE,
            config_dir: dir.to_string_lossy().into_owned(),
            claude_path: crate::engine::fake_opencode(),
            active: true,
            projects: Vec::new(),
            daily_budget: budget,
            ..Default::default()
        });
    });
    dir
}

/// A provider of paid models: $3 and $15 a million tokens in and out, $0.30 read from the cache.
fn paid() -> Value {
    json!({
        "providers": [{ "id": "paid", "name": "Paid", "source": "api", "models": { "coder": {
            "id": "coder", "providerID": "paid", "name": "Coder",
            "capabilities": { "input": { "text": true, "image": false } },
            "cost": { "input": 3, "output": 15, "cache_read": 0.3 },
            "limit": { "context": 200000, "output": 32000 }, "status": "active",
        } } }],
        "default": { "paid": "coder" },
    })
}

async fn agent_on(h: &Harness, project: &str, account: &str) -> String {
    let id = h
        .core
        .create_agent_with(
            project,
            AgentOptions {
                engine: Some(EngineId::OPENCODE),
                account: Some(account.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.core.ensure_process(&id).await.unwrap();
    id
}

#[tokio::test]
async fn an_opencode_turn_costs_its_steps_and_the_days_budget_pauses_its_tickets() {
    let h = harness("esc45-cost");
    let (p, _) = h.project(false).await;
    account_with(&h, "oc-paid", paid(), Some(0.004));
    let id = agent_on(&h, &p.id, "oc-paid").await;
    assert_eq!(h.agent(&id).model, "paid/coder");
    // Its account alone serves OpenCode (the Principal of OpenCode made meanwhile, off).
    h.set_settings(|s| {
        for a in &mut s.accounts {
            if a.id == "principal-opencode" {
                a.active = false;
            }
        }
    });
    // Under the budget: nothing holds its tickets back.
    assert!(h.core.project_hold(&p.id, EngineId::OPENCODE).is_none());
    // Two calls to the model: (1000 in, 50 out) then (100 in, 10 out, 20 read from the cache).
    h.turn(&id, "deux appels").await;
    let cost = (1000.0 * 3.0 + 50.0 * 15.0 + 100.0 * 3.0 + 10.0 * 15.0 + 20.0 * 0.3) / 1e6;
    let turn = h
        .items(&id)
        .into_iter()
        .rev()
        .find(|i| i["kind"] == "turn")
        .unwrap();
    assert!(
        (turn["cost"].as_f64().unwrap() - cost).abs() < 1e-12,
        "{turn}"
    );
    assert_eq!(turn["tokens"], 1000 + 50 + 100 + 10 + 20);
    let a = h.agent(&id);
    assert!((a.cost - cost).abs() < 1e-12);
    // Its window: its model's limit.
    assert_eq!(
        h.core.agent(&id).unwrap().lock().view().context_window,
        200_000
    );
    // The day's cost, of its account and in all.
    let usage = h.core.usage.lock().clone();
    let of = usage.account("oc-paid").unwrap();
    assert!((of.today_cost - cost).abs() < 1e-12);
    assert!((usage.today_cost - cost).abs() < 1e-12);
    assert_eq!(of.daily_budget, Some(0.004));
    // The statistics of OpenCode alone, and beside Claude Code's.
    let stats = h.core.stats_view_by("7d", None, Some("opencode"));
    assert!((stats.cost - cost).abs() < 1e-12);
    assert_eq!(stats.tokens, 1180);
    assert!(stats.engines.contains(&"opencode".to_string()));
    let all = h.core.stats_view_by("7d", None, None);
    let share = all
        .by_engine
        .iter()
        .find(|e| e.engine == "opencode")
        .unwrap();
    assert_eq!(share.turns, 1);
    assert!(h.core.stats_view_by("7d", None, Some("codex")).cost == 0.0);

    // Over the day's budget: its tickets wait for midnight, as at a quota used up; Claude Code's do not.
    let held = h
        .core
        .project_hold(&p.id, EngineId::OPENCODE)
        .expect("held");
    assert_eq!(held.pause.reason, PauseReason::Budget);
    assert_eq!(held.pause.until, crate::stats::day_bounds(now_ms()).1);
    assert!(h.core.project_hold(&p.id, EngineId::CLAUDE).is_none());
    // The budget raised in the account's settings: free again.
    let mut acct = h
        .core
        .settings
        .read()
        .accounts
        .iter()
        .find(|a| a.id == "oc-paid")
        .cloned()
        .unwrap();
    acct.daily_budget = Some(5.0);
    h.core.update_claude_account(acct.clone()).unwrap();
    assert!(h.core.project_hold(&p.id, EngineId::OPENCODE).is_none());
    // Nor a negative one, and 0 is none.
    acct.daily_budget = Some(-1.0);
    assert!(h.core.update_claude_account(acct.clone()).is_err());
    acct.daily_budget = Some(0.0);
    let saved = h.core.update_claude_account(acct).unwrap();
    assert_eq!(
        saved
            .iter()
            .find(|a| a.id == "oc-paid")
            .unwrap()
            .daily_budget,
        None
    );
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_local_model_costs_nothing_and_never_reaches_the_budget() {
    let h = harness("esc45-local");
    let (p, _) = h.project(false).await;
    // The built-in providers of the fake: Ollama's, without a price.
    let dir = h.dir.join("oc-local");
    std::fs::create_dir_all(&dir).unwrap();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "oc-local".into(),
            name: "Local".into(),
            engine: EngineId::OPENCODE,
            config_dir: dir.to_string_lossy().into_owned(),
            claude_path: crate::engine::fake_opencode(),
            active: true,
            projects: Vec::new(),
            daily_budget: Some(0.01),
            ..Default::default()
        });
    });
    let id = agent_on(&h, &p.id, "oc-local").await;
    h.set_settings(|s| {
        for a in &mut s.accounts {
            if a.id == "principal-opencode" {
                a.active = false;
            }
        }
    });
    h.turn(&id, "deux appels").await;
    assert_eq!(h.agent(&id).cost, 0.0);
    assert_eq!(
        h.core.agent(&id).unwrap().lock().view().context_window,
        32768
    );
    assert!(h.core.project_hold(&p.id, EngineId::OPENCODE).is_none());
    h.core.stop_for_good();
}

/// What the agent's last call of `whoami` (Escouade's test tool, through OpenCode) answered, if it
/// made one: the caller Escouade's server took it for.
fn whoami(h: &Harness, id: &str) -> Option<Value> {
    let items = h.items(id);
    let call = items
        .iter()
        .rev()
        .find(|i| i["name"] == "mcp__escouade__whoami")?;
    let text = serde_json::to_string(call).unwrap();
    let at = text.find("caller")?;
    let start = text[..at].rfind('{')?;
    let end = start + text[start..].find('}')? + 1;
    serde_json::from_str(&text[start..end].replace("\\\"", "\"")).ok()
}

#[tokio::test]
async fn escouades_server_is_given_to_each_agent_in_its_folder_with_its_own_token_or_kept_out() {
    let h = harness("esc45-mcp");
    let (p1, _) = h.project(false).await;
    let p2 = h.second_project().await;
    let dir = account_with(&h, "oc-mcp", paid(), None);
    // The switch off: an agent of the second project starts without Escouade's server.
    let refused = agent_on(&h, &p2.id, "oc-mcp").await;
    h.turn(&refused, "outil escouade whoami").await;
    assert!(texts(&h, &refused).contains(&"Escouade n’est pas là.".to_string()));

    // On: an agent of the first project, on the same account (the same server), reaches it as
    // itself, with its own token.
    h.core.set_mcp_enabled(true).unwrap();
    let allowed = agent_on(&h, &p1.id, "oc-mcp").await;
    h.turn(&allowed, "outil escouade whoami").await;
    assert_eq!(
        whoami(&h, &allowed),
        Some(json!({ "caller": "agent", "agent": allowed })),
        "{:#?}",
        h.items(&allowed)
    );
    let serves = |dir: &std::path::Path| {
        launches(dir)
            .iter()
            .filter(|l| l["argv"][0] == "serve")
            .count()
    };
    assert_eq!(serves(&dir), 1, "one server for both");
    // The server's config keeps it out of every folder; only the allowed agent's has it.
    let config = &launches(&dir)
        .into_iter()
        .find(|l| l["argv"][0] == "serve")
        .unwrap()["config"];
    assert_eq!(config["mcp"]["escouade"]["enabled"], false);
    assert!(!config.to_string().contains("Bearer"), "{config}");
    h.turn(&refused, "outil escouade whoami").await;
    assert_eq!(
        texts(&h, &refused)
            .iter()
            .filter(|t| *t == "Escouade n’est pas là.")
            .count(),
        2
    );

    // A second agent in the same folder (no worktrees): a server of its own, its own token.
    let other = agent_on(&h, &p1.id, "oc-mcp").await;
    assert_eq!(h.agent(&other).cwd, h.agent(&allowed).cwd);
    h.turn(&other, "outil escouade whoami").await;
    assert_eq!(
        whoami(&h, &other),
        Some(json!({ "caller": "agent", "agent": other }))
    );
    assert_eq!(serves(&dir), 2);
    h.turn(&allowed, "outil escouade whoami").await;
    assert_eq!(
        whoami(&h, &allowed),
        Some(json!({ "caller": "agent", "agent": allowed }))
    );
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_texts_written_by_opencode_go_through_opencode_run_on_a_model_always_given() {
    let h = harness("esc45-writer");
    let (p, _) = h.project(false).await;
    let mut providers = paid();
    providers["providers"][0]["models"]["fast"] = json!({
        "id": "fast", "providerID": "paid", "name": "Fast", "limit": { "context": 100000 },
    });
    let dir = account_with(&h, "oc-writer", providers, None);
    h.set_settings(|s| {
        s.writer_engine = Some(EngineId::OPENCODE);
        for a in &mut s.accounts {
            if a.engine == EngineId::OPENCODE && a.id != "oc-writer" {
                a.active = false;
            }
        }
    });
    let runs = || -> Vec<Value> {
        launches(&dir)
            .into_iter()
            .filter_map(|l| l.get("run").cloned())
            .collect()
    };
    // No model chosen: the default one of its providers, listed without a server.
    let answer = h
        .core
        .one_shot("Nomme la tâche.", "ajouter un bouton", None, Some(&p.id))
        .await
        .unwrap();
    assert_eq!(
        answer,
        "Réponse d’OpenCode (paid/coder) : ajouter un bouton"
    );
    let asked = runs();
    assert_eq!(asked.len(), 1);
    let run = &asked[0];
    let argv: Vec<&str> = run["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert!(
        argv.windows(2).any(|w| w == ["--model", "paid/coder"]),
        "{argv:?}"
    );
    assert!(
        argv.windows(2).any(|w| w == ["--format", "json"]),
        "{argv:?}"
    );
    // The question by its input, the instructions as the prompt of Escouade's agent, without a tool.
    assert_eq!(run["prompt"], "ajouter un bouton");
    assert!(!argv.iter().any(|a| a.contains("ajouter")), "{argv:?}");
    let config = &run["config"];
    assert_eq!(config["model"], "paid/coder");
    assert_eq!(config["small_model"], "paid/coder");
    assert_eq!(
        config["agent"]["escouade-writer"]["prompt"],
        "Nomme la tâche."
    );
    assert_eq!(config["agent"]["escouade-writer"]["tools"]["*"], false);
    assert_eq!(config["mcp"]["escouade"]["enabled"], false);
    // Its session deleted from OpenCode's history.
    let deleted = |n: usize| {
        launches(&dir)
            .iter()
            .filter(|l| l["argv"][0] == "session" && l["argv"][1] == "delete")
            .count()
            == n
    };
    assert!(deleted(1), "{:?}", launches(&dir));

    // The model chosen in the settings.
    h.set_settings(|s| s.writer_model = "paid/fast".into());
    let answer = h
        .core
        .one_shot("Nomme.", "autre", None, Some(&p.id))
        .await
        .unwrap();
    assert_eq!(answer, "Réponse d’OpenCode (paid/fast) : autre");
    // One of no provider any more: the default one again.
    h.set_settings(|s| s.writer_model = "gone/model".into());
    let answer = h
        .core
        .one_shot("Nomme.", "encore", None, Some(&p.id))
        .await
        .unwrap();
    assert!(
        answer.starts_with("Réponse d’OpenCode (paid/coder)"),
        "{answer}"
    );
    // OpenCode's failure is told.
    let e = h
        .core
        .one_shot("Nomme.", "échoue", None, Some(&p.id))
        .await
        .unwrap_err();
    assert!(
        format!("{e:#}").contains("Unexpected server error"),
        "{e:#}"
    );
    assert!(deleted(4));

    // No provider connected: no text, and no `opencode run` asked (OpenCode would pick a model itself).
    std::fs::write(
        dir.join("providers.json"),
        r#"{ "providers": [], "default": {} }"#,
    )
    .unwrap();
    h.core.engine_models.write().remove(&EngineId::OPENCODE);
    let before = runs().len();
    let e = h
        .core
        .one_shot("Nomme.", "rien", None, Some(&p.id))
        .await
        .unwrap_err();
    assert!(
        format!("{e:#}").contains("Aucun modèle d’OpenCode"),
        "{e:#}"
    );
    assert_eq!(runs().len(), before);
    h.core.stop_for_good();
}

/// Real run (ESC-45), as `real_opencode` (`ESCOUADE_REAL_OPENCODE`, `XDG_*`, Ollama's
/// `qwen3-coder:30b-32k`): a text written by `opencode run` (its session gone from OpenCode's
/// history afterwards), then an agent that calls Escouade's test tool `whoami` through the entry of
/// its folder and is taken for itself. By hand only:
/// `cargo test --lib real_opencode_writes -- --ignored --nocapture`.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_opencode_writes_and_drives_escouade() {
    let real = std::env::var("ESCOUADE_REAL_OPENCODE").expect("ESCOUADE_REAL_OPENCODE");
    assert!(std::env::var("XDG_CONFIG_HOME").is_ok(), "XDG_*");
    let h = harness("esc45-real");
    let (p, _) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-opencode".into(),
            name: "Principal".into(),
            engine: EngineId::OPENCODE,
            config_dir: String::new(),
            claude_path: real.clone(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            ..Default::default()
        });
        s.writer_engine = Some(EngineId::OPENCODE);
        s.writer_model = "ollama/qwen3-coder:30b-32k".into();
    });
    let sessions = || {
        let out = std::process::Command::new(&real)
            .args(["session", "list"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).matches("ses_").count()
    };
    let before = sessions();
    let name = h
        .core
        .one_shot(
            "Answer with a short kebab-case name of 2 to 4 words for the task, nothing else.",
            "Corrige le bouton Enregistrer qui ne sauve pas les réglages",
            None,
            Some(&p.id),
        )
        .await
        .unwrap();
    println!("NAME {name}");
    assert!(!name.is_empty() && name.len() < 80, "{name}");
    assert_eq!(sessions(), before, "its session deleted");

    h.core.set_mcp_enabled(true).unwrap();
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::OPENCODE),
                model: Some("ollama/qwen3-coder:30b-32k".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.core
        .send_message(
            &id,
            "Call the tool escouade_whoami once, then repeat exactly what it returned.".into(),
            vec![],
        )
        .await
        .unwrap();
    wait_long(&h, "the call of whoami", 900, |h| {
        whoami(h, &id).is_some() || h.items(&id).iter().any(|i| i["kind"] == "turn")
    })
    .await;
    println!("WHOAMI {:?}", whoami(&h, &id));
    println!("ITEMS {:#?}", h.items(&id));
    assert_eq!(
        whoami(&h, &id),
        Some(json!({ "caller": "agent", "agent": id }))
    );
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_principal_of_opencode_takes_a_budget_though_claude_codes_is_named_principal_too() {
    let h = harness("esc45-principal");
    let (p, _) = h.project(false).await;
    opencode_account(&h);
    let _id = new_agent(&h, &p.id).await;
    let mut principal = h
        .core
        .settings
        .read()
        .accounts
        .iter()
        .find(|a| a.id == "principal-opencode")
        .cloned()
        .unwrap();
    assert_eq!(principal.name, "Principal");
    // Made at its first agent, its usage with it: the status bar shows its cost of the day.
    let usage = h.core.usage.lock().clone();
    let of = usage.account("principal-opencode").expect("its usage");
    assert_eq!(of.engine, EngineId::OPENCODE);
    principal.daily_budget = Some(3.0);
    let saved = h.core.update_claude_account(principal.clone()).unwrap();
    let got = saved.iter().find(|a| a.id == "principal-opencode").unwrap();
    assert_eq!(got.daily_budget, Some(3.0));
    assert_eq!(
        h.core
            .usage
            .lock()
            .account("principal-opencode")
            .unwrap()
            .daily_budget,
        Some(3.0)
    );
    // Another account of Claude Code may still not take its name.
    principal.name = "principal".into();
    assert!(h.core.update_claude_account(principal).is_ok());
    h.core.stop_for_good();
}

/// Whether a session of the fake `opencode` under `dir` has the title `title`.
fn titled(dir: &std::path::Path, title: &str) -> bool {
    let wanted = format!("\"title\":{}", serde_json::to_string(title).unwrap());
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(|e| e.ok()).any(|e| {
        let path = e.path();
        if path.is_dir() {
            titled(&path, title)
        } else {
            std::fs::read_to_string(&path).is_ok_and(|t| t.contains(&wanted))
        }
    })
}

/// ESC-156 with OpenCode: an agent of OpenCode, named by OpenCode (« Modèle des textes rédigés »),
/// gets its worktree at its first message under that name, and its session takes it.
#[tokio::test]
async fn an_opencode_agent_gets_its_worktree_at_its_first_message_under_the_name_opencode_gives() {
    let h = harness("esc156-opencode");
    let (p, _) = h.project_as_new(true).await;
    let dir = account_with(&h, "oc", paid(), None);
    h.set_settings(|s| {
        s.writer_engine = Some(EngineId::OPENCODE);
        for a in &mut s.accounts {
            if a.engine == EngineId::OPENCODE && a.id != "oc" {
                a.active = false;
            }
        }
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::OPENCODE),
                account: Some("oc".into()),
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
    assert_eq!(m.name, "ajoute-un-opencode");
    let wt = m.worktree.unwrap();
    assert_eq!(
        std::path::PathBuf::from(&wt.path),
        p.worktrees_root().join("ajoute-un-opencode")
    );
    assert_eq!(wt.branch, "escouade/ajoute-un-opencode");
    h.wait("its session renamed", |_| {
        titled(&dir, "ajoute-un-opencode")
    })
    .await;
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_models_of_an_opencode_account_are_read_without_an_agent_and_kept_after_a_restart() {
    let h = harness("esc186-opencode-models");
    opencode_account(&h);
    let menu = |core: &crate::core::Core<tauri::test::MockRuntime>| -> Vec<String> {
        core.account_models
            .read()
            .get("principal-opencode")
            .map(|m| m.catalog.iter().map(|m| m.id.clone()).collect())
            .unwrap_or_default()
    };
    assert!(menu(&h.core).is_empty());
    h.core.read_account_models("principal-opencode").await;
    assert_eq!(
        menu(&h.core),
        [
            "ollama/gpt-oss:20b-32k",
            "ollama/qwen3-coder:30b-32k",
            "lmstudio/devstral-small"
        ]
    );
    assert!(h.core.agents.read().is_empty());
    h.core.save_now();
    let app = tauri::test::mock_app();
    let (reloaded, _rx) = crate::core::Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(menu(&reloaded).len(), 3);
    h.core.stop_for_good();
}

/// ESC-205: `/compact` typed to an agent of OpenCode is no message to its model. OpenCode
/// compacts its session (`POST /session/:id/summarize`, as its own `/compact` does), the
/// conversation says when it starts and when it ends, the context gauge holds its summary alone,
/// and the agent is at rest again once its session is.
#[tokio::test]
async fn compact_typed_to_an_opencode_agent_summarizes_its_session() {
    let h = harness("esc205-opencode");
    let (p, _) = h.project(false).await;
    opencode_account(&h);
    let id = new_agent(&h, &p.id).await;
    h.turn(&id, "bonjour").await;
    let context = |h: &Harness| h.core.agent(&id).unwrap().lock().context_tokens;
    let before = context(&h);
    assert!(before > 0);
    h.turn(&id, "/compact").await;
    let all = texts(&h, &id);
    assert!(
        all.contains(&"Résumé de la conversation (summarize).".to_string()),
        "{all:?}"
    );
    assert!(
        !all.iter().any(|t| t.contains("tu as dit : /compact")),
        "the model was told /compact: {all:?}"
    );
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
            "user: /compact",
            "notice: Compaction du contexte…",
            "notice: Contexte compacté",
        ],
        "{shown:?}"
    );
    assert!(context(&h) < before, "{before} → {}", context(&h));
    assert!(!h.agent(&id).status.is_active());
    // An instruction after it: OpenCode's request takes none.
    let e = h
        .core
        .send_message(&id, "/compact garde les décisions".into(), vec![])
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("instruction"), "{e:#}");
    h.core.stop_for_good();
}
