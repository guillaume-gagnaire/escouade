//! The handover to another harness (ESC-88): its file, its mode; and, run by the core, Claude
//! Code's fake stopped by the limit going on on the minimal fake of the bench, and back.

use super::*;
use crate::core_tests::{agent_on, harness, repo, Harness};
use crate::model::{Account, AgentMeta, Project};
use crate::plan::PlanTask;
use crate::stats::{EngineShare, Labels};
use std::path::Path;
use std::sync::atomic::Ordering;

// ---------- the relay of « Vue Cursor » ----------

fn line_of(kind: &str, text: &str) -> Value {
    json!({ "kind": kind, "id": crate::model::new_id(), "text": text })
}

#[test]
fn a_relay_tells_the_exchange_in_its_order_with_the_tools_and_never_a_subagents_words() {
    let items = vec![
        line_of("user", "Ajoute un test"),
        json!({ "kind": "tool", "name": "Bash", "input": { "command": "npm test" } }),
        json!({ "kind": "tool", "name": "Read", "input": { "file_path": "src/a.ts" } }),
        json!({ "kind": "text", "text": "Le rapport du sous-agent", "parent": "call-1" }),
        line_of("text", "Test ajouté."),
        json!({ "kind": "turn", "cost": 0.0 }),
        line_of("user", "Merci"),
    ];
    let text = relay(Lang::Fr, Relay::ToInterface, &items, None, RELAY_BUDGET).unwrap();
    assert!(
        text.starts_with("# Conversation d’avant, dans Escouade"),
        "{text}"
    );
    let at = |s: &str| text.find(s).unwrap_or_else(|| panic!("{s} in {text}"));
    assert!(at("- Utilisateur : Ajoute un test") < at("(outils : Bash npm test ; Read src/a.ts)"));
    assert!(at("(outils :") < at("- Agent : Test ajouté."));
    assert!(at("- Agent : Test ajouté.") < at("- Utilisateur : Merci"));
    assert!(!text.contains("sous-agent"));
    // The other way, in English: its own words, the same exchange.
    let back = relay(Lang::En, Relay::ToSession, &items, None, RELAY_BUDGET).unwrap();
    assert!(
        back.starts_with("# What was said in Cursor’s interface"),
        "{back}"
    );
    assert!(back.contains("- User: Ajoute un test") && back.contains("- Agent: Test ajouté."));
}

#[test]
fn a_relay_of_nothing_said_is_none_and_a_long_one_keeps_the_latest_lines() {
    let tools = [json!({ "kind": "tool", "name": "Bash", "input": { "command": "ls" } })];
    assert_eq!(
        relay(Lang::Fr, Relay::ToSession, &tools, None, RELAY_BUDGET),
        None
    );
    assert_eq!(
        relay(Lang::Fr, Relay::ToInterface, &[], None, RELAY_BUDGET),
        None
    );
    let many: Vec<Value> = (0..200)
        .map(|n| line_of("user", &format!("Message numéro {n} {}", "x".repeat(100))))
        .collect();
    let text = relay(Lang::Fr, Relay::ToSession, &many, None, 3_000).unwrap();
    assert!(text.chars().count() <= 3_000);
    assert!(text.contains("Message numéro 199"));
    assert!(!text.contains("Message numéro 0 "));
    assert!(text.contains("ligne(s) plus ancienne(s) omise(s)"));
}

#[test]
fn a_relay_into_the_interface_gives_the_tasks_of_the_plan() {
    let mut plan = PlanState::default();
    plan.tasks.push(PlanTask {
        title: "Écrire le test".into(),
        status: TaskStatus::Done,
        ..Default::default()
    });
    let items = [line_of("user", "Go")];
    let text = relay(
        Lang::Fr,
        Relay::ToInterface,
        &items,
        Some(&plan),
        RELAY_BUDGET,
    )
    .unwrap();
    assert!(
        text.contains("Tâches du plan :\n- [x] Écrire le test"),
        "{text}"
    );
    let back = relay(
        Lang::Fr,
        Relay::ToSession,
        &items,
        Some(&plan),
        RELAY_BUDGET,
    )
    .unwrap();
    assert!(!back.contains("Tâches du plan"));
}

// ---------- the mode ----------

#[test]
fn the_mode_after_a_handover_is_the_same_else_the_closest_that_never_allows_more() {
    let all = [
        "auto",
        "default",
        "plan",
        "acceptEdits",
        "bypassPermissions",
    ];
    for m in all {
        assert_eq!(closest_mode(m, &all).as_deref(), Some(m));
    }
    // Only « Demander »: everything above it comes down to it, the plan mode has nothing as strict.
    let minimal = ["default"];
    assert_eq!(
        closest_mode("bypassPermissions", &minimal).as_deref(),
        Some("default")
    );
    assert_eq!(
        closest_mode("acceptEdits", &minimal).as_deref(),
        Some("default")
    );
    assert_eq!(closest_mode("plan", &minimal), None);
    // No `auto` (a model without it): the next one down, never « Sans permissions ».
    let no_auto = ["default", "plan", "acceptEdits", "bypassPermissions"];
    assert_eq!(
        closest_mode("auto", &no_auto).as_deref(),
        Some("acceptEdits")
    );
    // A mode it does not know counts as « Demander ».
    assert_eq!(closest_mode("dontAsk", &all).as_deref(), Some("default"));
    // Never more permissive, whatever is offered.
    for (i, m) in MODES.iter().enumerate() {
        for offered in [&all[..], &minimal[..], &no_auto[..], &["plan"][..]] {
            if let Some(got) = closest_mode(m, offered) {
                assert!(rank(&got).unwrap() <= i, "{m} → {got}");
            }
        }
    }
}

#[test]
fn the_modes_of_a_harness_hold_auto_only_where_its_model_takes_it() {
    let claude = crate::engine::by_id(EngineId::CLAUDE);
    assert!(modes_of(claude, true).contains(&"auto"));
    let without = modes_of(claude, false);
    assert!(!without.contains(&"auto") && without.contains(&"default"));
}

#[test]
fn the_handover_model_is_the_one_set_for_the_harness_else_its_best() {
    let claude = engine::by_id(EngineId::CLAUDE);
    let mut s = Settings::default();
    assert_eq!(model_for(&s, claude), "fable");
    s.handover_models.insert(EngineId::CLAUDE, "sonnet".into());
    assert_eq!(model_for(&s, claude), "sonnet");
    // One it does not list: its best.
    s.handover_models.insert(EngineId::CLAUDE, "gpt-9".into());
    assert_eq!(model_for(&s, claude), "fable");
    assert_eq!(model_for(&s, engine::by_id(EngineId::BENCH)), "grand");
}

// ---------- the file ----------

fn user(text: &str) -> Value {
    json!({ "kind": "user", "id": crate::model::new_id(), "text": text })
}

fn said(text: &str) -> Value {
    json!({ "kind": "text", "id": crate::model::new_id(), "text": text, "parent": null })
}

fn turn(limited: bool) -> Value {
    json!({ "kind": "turn", "id": crate::model::new_id(), "limited": limited, "isError": limited, "interrupted": false })
}

fn changes() -> Changes {
    Changes {
        status: " M src/app.ts\n?? src/new.ts".into(),
        stat: " src/app.ts | 2 +-\n 1 file changed".into(),
        commits: "abc1234 feat: le début".into(),
    }
}

fn dossier_of(items: &[Value], plan: Option<&PlanState>, budget: usize) -> String {
    dossier(&Dossier {
        lang: Lang::Fr,
        from: "Claude Code",
        limited: true,
        items,
        plan,
        progress: Some("Tests écrits, reste l’interface"),
        changes: &changes(),
        ticket: Some("ESC-88 — Continuer sur un autre harness"),
        budget,
    })
}

#[test]
fn the_handover_file_tells_the_requests_the_work_the_tasks_the_files_the_cut_turn_and_the_ticket() {
    let items = vec![
        user("Ajoute la passation entre harness"),
        said("Je commence par le modèle."),
        json!({ "kind": "tool", "id": "t0", "name": "Edit", "input": { "file_path": "src/model.rs" }, "status": "ok" }),
        turn(false),
        user("Et les tests aussi"),
        // A note after a turn is no turn.
        json!({ "kind": "notice", "id": "n1", "text": "Reprise automatique" }),
        said("Les tests avancent."),
        json!({ "kind": "tool", "id": "t1", "name": "Bash", "input": { "command": "cargo test" }, "status": "interrupted" }),
        json!({ "kind": "permission", "id": "p1", "toolName": "Bash", "input": { "command": "rm -rf target" }, "decision": null }),
        turn(true),
        json!({ "kind": "notice", "id": "n2", "text": "Limite" }),
    ];
    let plan = PlanState {
        tasks: vec![
            PlanTask {
                id: "1".into(),
                title: "Écrire le modèle".into(),
                status: TaskStatus::Done,
                ..Default::default()
            },
            PlanTask {
                id: "2".into(),
                title: "Écrire les tests".into(),
                status: TaskStatus::InProgress,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let text = dossier_of(&items, Some(&plan), budget(200_000));
    for part in [
        "# Passation : suite d’un travail commencé sur Claude Code",
        "son quota Claude Code est épuisé",
        "ticket ESC-88 — Continuer sur un autre harness",
        "## Demande d’origine\n\nAjoute la passation entre harness",
        "- Et les tests aussi",
        "Avancement annoncé : Tests écrits, reste l’interface",
        "- [x] Écrire le modèle",
        "- [~] Écrire les tests",
        "> Je commence par le modèle.",
        "> Les tests avancent.",
        "abc1234 feat: le début",
        " M src/app.ts",
        "1 file changed",
        "Coupé par la limite d’usage. Dernier outil : Bash cargo test (interrupted).",
        "- Permission demandée pour Bash :  rm -rf target",
    ] {
        assert!(text.contains(part), "{part:?} missing from:\n{text}");
    }
    // The latest message of the agent comes after the older one.
    assert!(text.find("Je commence").unwrap() < text.find("Les tests avancent").unwrap());
}

#[test]
fn a_handover_file_sent_before_is_not_one_of_the_users_requests_and_a_turn_ended_well_is_not_cut() {
    let mut back = user("# Passation : …");
    back["origin"] = json!("handover");
    let items = vec![
        user("Fais-le"),
        said("Fait."),
        turn(false),
        back,
        said("Repris."),
        turn(false),
    ];
    let text = dossier_of(&items, None, budget(200_000));
    assert!(!text.contains("Messages suivants"), "{text}");
    assert!(!text.contains("Dernier tour coupé"), "{text}");
    // A turn its process left before it ended is cut.
    let items = vec![user("Fais-le"), said("Je commence…")];
    let text = dossier_of(&items, None, budget(200_000));
    assert!(text.contains("Coupé avant sa fin."), "{text}");
}

#[test]
fn the_handover_file_is_bounded_by_the_window_of_its_model_keeping_the_first_request_and_the_latest_work(
) {
    let mut items = vec![user("La demande d’origine")];
    for n in 0..200 {
        items.push(user(&format!("Message {n} {}", "x".repeat(500))));
        items.push(said(&format!("Réponse {n} {}", "y".repeat(500))));
        items.push(turn(false));
    }
    let max = budget(4_000);
    let text = dossier_of(&items, None, max);
    assert!(text.chars().count() <= max, "{}", text.chars().count());
    assert!(text.contains("La demande d’origine"));
    assert!(text.contains("Réponse 199"), "{text}");
    assert!(text.contains("plus ancien(s) omis"), "{text}");
    // A big window takes much more of it.
    let big = dossier_of(&items, None, budget(200_000));
    assert!(big.chars().count() > text.chars().count() * 3);
}

// ---------- run by the core, on two fakes ----------

/// A Claude account of the test (its folder its own), reserved to `projects` (none: all).
fn claude_account(h: &Harness, id: &str, projects: &[&str]) -> Account {
    Account {
        id: id.into(),
        name: id.into(),
        config_dir: h.dir.join(format!("claude-{id}")).to_string_lossy().into(),
        projects: projects.iter().map(|p| p.to_string()).collect(),
        ..Default::default()
    }
}

/// Two projects: P, which a Minimal account (the bench's fake) serves, and Q, which none does.
/// Principal is reserved to Q; « Pro », a Claude account of both, is out of quota for good (its
/// folder holds a `fake-limit`): for P, Claude Code is spent once Pro is.
async fn two_projects(h: &Harness) -> (Project, Project) {
    let (p, _) = h.project(true).await;
    let r = repo(&h.dir.join("q"));
    let q = h
        .core
        .create_project(
            &r.to_string_lossy(),
            "autre",
            "oklch(0.7 0.1 200)",
            true,
            None,
            true,
        )
        .await
        .unwrap();
    let pro = claude_account(h, "pro", &[]);
    std::fs::create_dir_all(&pro.config_dir).unwrap();
    std::fs::write(Path::new(&pro.config_dir).join("fake-limit"), "").unwrap();
    let mut s = h.core.settings.read().clone();
    s.accounts[0].projects = vec![q.id.clone()];
    s.accounts.push(pro);
    s.accounts.push(Account {
        id: "relais".into(),
        name: "Relais".into(),
        engine: EngineId::BENCH,
        config_dir: h.dir.join("relais").to_string_lossy().into(),
        projects: vec![p.id.clone()],
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    (p, q)
}

/// Sends `text` and waits for the turn to be over, however it ends.
async fn turn_over(h: &Harness, id: &str, text: &str) {
    let before = h.turns(id).len();
    h.core
        .send_message(id, text.to_string(), vec![])
        .await
        .unwrap();
    h.wait("the turn over", |h| {
        h.turns(id).len() > before && !h.agent(id).status.is_active()
    })
    .await;
}

impl Harness {
    fn turns(&self, id: &str) -> Vec<Value> {
        self.items(id)
            .into_iter()
            .filter(|i| i["kind"] == "turn")
            .collect()
    }

    /// The handover file the agent was sent last.
    fn dossier(&self, id: &str) -> Option<String> {
        self.items(id)
            .into_iter()
            .rev()
            .find(|i| i["kind"] == "user" && i["origin"] == "handover")
            .and_then(|i| i["text"].as_str().map(str::to_string))
    }
}

#[tokio::test]
async fn an_agent_stopped_by_the_limit_goes_on_on_the_other_fake_harness_with_its_handover_file_then_back(
) {
    let h = harness("handover-fakes");
    let (p, q) = two_projects(&h).await;
    let a = agent_on(&h, &p, "pro").await;
    let id = a.id.clone();
    let cwd = a.cwd.clone();
    // Its work in its folder: a file changed, not committed.
    std::fs::write(Path::new(&cwd).join("src").join("app.ts"), "const a = 2;\n").unwrap();
    turn_over(&h, &id, "Ajoute la passation entre harness").await;
    assert_eq!(h.turns(&id).last().unwrap()["limited"], true);
    // Claude Code is spent for P (Principal does not serve it): Minimal is its fallback harness.
    assert_eq!(h.core.handover_targets(&id).unwrap(), [EngineId::BENCH]);
    // Off by default: nothing goes by itself.
    assert!(!h.core.settings.read().handover_on_limit);
    assert_eq!(h.agent(&id).engine, EngineId::CLAUDE);

    // Named after its first message (its branch with it) before it goes on.
    h.wait("its name", |h| h.agent(&id).named).await;
    let a = h.agent(&id);
    // Never during a turn.
    h.core.agent(&id).unwrap().lock().meta.status = AgentStatus::Running;
    let refused = h
        .core
        .continue_on_engine(&id, EngineId::BENCH)
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.starts_with("L’agent travaille encore"), "{refused}");
    h.core.agent(&id).unwrap().lock().meta.status = a.status;
    assert_eq!(h.agent(&id).engine, EngineId::CLAUDE);

    h.core
        .continue_on_engine(&id, EngineId::BENCH)
        .await
        .unwrap();

    // The same agent, its folder and branch, on Minimal's account, a new session.
    let m = h.agent(&id);
    assert_eq!((m.engine, m.account.as_str()), (EngineId::BENCH, "relais"));
    let branch = |m: &AgentMeta| {
        m.worktree
            .as_ref()
            .map(|w| (w.path.clone(), w.branch.clone()))
    };
    assert_eq!((m.cwd.as_str(), branch(&m)), (cwd.as_str(), branch(&a)));
    assert!(branch(&m).is_some());
    assert_eq!(m.model, "grand");
    // Its mode allows no more than before (Minimal only has « Demander »).
    assert_eq!(m.mode, "default");
    let left = m.handed_from.clone().unwrap();
    assert_eq!(
        (left.engine, left.account.as_str()),
        (EngineId::CLAUDE, "pro")
    );
    let until = left.until.expect("the reset it waits for");
    h.wait("Minimal's reply to the file", |h| {
        h.turns(&id).last().is_some_and(|t| t["engine"] == "bench")
            && !h.agent(&id).status.is_active()
    })
    .await;
    assert!(h.agent(&id).session_id.unwrap().starts_with("min-"));
    // The file, built by Escouade without Claude Code: the request, the files, the cut turn.
    let file = h.dossier(&id).unwrap();
    for part in [
        "suite d’un travail commencé sur Claude Code",
        "Ajoute la passation entre harness",
        "M src/app.ts",
        "1 file changed",
        "Coupé par la limite d’usage",
    ] {
        assert!(file.contains(part), "{part:?} missing from:\n{file}");
    }
    // The conversation marks the change, and each turn keeps its harness.
    let sep = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "handover")
        .unwrap();
    assert_eq!(
        (&sep["from"], &sep["to"], &sep["limited"], &sep["until"]),
        (
            &json!("claude"),
            &json!("bench"),
            &json!(true),
            &json!(until)
        )
    );
    let engines: Vec<Value> = h.turns(&id).iter().map(|t| t["engine"].clone()).collect();
    assert_eq!(engines, [json!("claude"), json!("bench")]);

    // Claude Code is set aside until its reset: not offered, refused.
    assert!(h.core.handover_targets(&id).unwrap().is_empty());
    let refused = h
        .core
        .continue_on_engine(&id, EngineId::CLAUDE)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        refused.starts_with("Le quota Claude Code n’est pas revenu"),
        "{refused}"
    );
    // Its quota back: offered, never taken by itself.
    std::fs::remove_file(h.dir.join("claude-pro").join("fake-limit")).unwrap();
    h.core
        .clock_ahead
        .store(until - crate::model::now_ms() + 60_000, Ordering::SeqCst);
    assert_eq!(h.core.handover_targets(&id).unwrap(), [EngineId::CLAUDE]);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(h.agent(&id).engine, EngineId::BENCH);

    // « Revenir sur Claude Code »: a handover the other way, what Minimal did in its file. On haiku,
    // which the `claude` of Pro told runs Haiku 4.5, without the mode `auto` (ESC-121): an agent
    // in « Auto » goes in « Édits auto », the closest that allows no more.
    h.core
        .settings
        .write()
        .handover_models
        .insert(EngineId::CLAUDE, "haiku".into());
    h.core.account_models.write().insert(
        "pro".into(),
        crate::model::AccountModels::new(
            "2.1.292".into(),
            crate::model::ModelInfo::list(&json!([
                { "value": "sonnet", "resolvedModel": "claude-sonnet-5-5", "supportsAutoMode": true },
                { "value": "haiku", "resolvedModel": "claude-haiku-4-5-20251001" },
            ])),
        ),
    );
    h.core.agent(&id).unwrap().lock().meta.mode = "auto".into();
    h.core
        .continue_on_engine(&id, EngineId::CLAUDE)
        .await
        .unwrap();
    let m = h.agent(&id);
    assert_eq!((m.engine, m.account.as_str()), (EngineId::CLAUDE, "pro"));
    assert_eq!(m.handed_from, None);
    assert_eq!(
        (m.model.as_str(), m.mode.as_str()),
        ("haiku", "acceptEdits")
    );
    h.wait("Claude's turn after the way back", |h| {
        h.turns(&id).len() == 3 && !h.agent(&id).status.is_active()
    })
    .await;
    let back = h.dossier(&id).unwrap();
    assert!(back.contains("commencé sur Minimal"), "{back}");
    assert!(back.contains("l’utilisateur l’a demandé"), "{back}");
    // What Minimal said (the file names « Fichiers modifiés », its scenario of a file changed).
    assert!(back.contains("> Fichier modifié."), "{back}");
    // The first file is no request of the user's.
    assert!(!back.contains("Messages suivants"), "{back}");
    let sent = h.stdin_messages(Path::new(&cwd));
    let last = sent.last().unwrap()["message"]["content"].to_string();
    assert!(last.contains("Passation"), "{last}");
    assert_eq!(h.turns(&id).last().unwrap()["engine"], "claude");
    // Its turns counted by the harness that ran each.
    h.wait("the turns counted by harness", |h| {
        let shares: std::collections::BTreeSet<String> = h
            .core
            .stats
            .query("day", None, &Labels::default())
            .by_engine
            .into_iter()
            .map(|s: EngineShare| s.engine)
            .collect();
        shares.into_iter().collect::<Vec<_>>() == ["bench", "claude"]
    })
    .await;

    // Q: no account of Minimal serves it (« Réservé aux projets »), Principal still does.
    let other = agent_on(&h, &q, "pro").await.id;
    std::fs::write(h.dir.join("claude-pro").join("fake-limit"), "").unwrap();
    turn_over(&h, &other, "la limite").await;
    assert!(h.core.handover_targets(&other).unwrap().is_empty());
    let refused = h
        .core
        .continue_on_engine(&other, EngineId::BENCH)
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(
        refused,
        "Aucun compte Minimal actif ne sert le projet de cet agent."
    );
    assert_eq!(h.agent(&other).engine, EngineId::CLAUDE);
}

/// A meta of the agent `id`, once `pred` holds of it.
async fn agent_when(
    h: &Harness,
    id: &str,
    what: &str,
    pred: impl Fn(&AgentMeta) -> bool,
) -> AgentMeta {
    h.wait(what, |h| pred(&h.agent(id))).await;
    h.agent(id)
}

#[tokio::test]
async fn with_the_option_an_agent_whose_harness_is_spent_goes_on_by_itself_and_its_ticket_with_it()
{
    let h = harness("handover-auto");
    let (p, q) = two_projects(&h).await;
    h.core
        .save_settings({
            let mut s = h.core.settings.read().clone();
            s.handover_on_limit = true;
            s
        })
        .unwrap();
    // An agent of P, stopped by the limit: on Minimal by itself, its file sent.
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    let m = agent_when(&h, &id, "the handover", |m| m.engine == EngineId::BENCH).await;
    assert_eq!(
        m.handed_from.as_ref().map(|l| l.engine),
        Some(EngineId::CLAUDE)
    );
    h.wait("Minimal's reply", |h| {
        h.turns(&id).last().is_some_and(|t| t["engine"] == "bench")
    })
    .await;
    // An agent of Q has no harness to go on on: it waits for the reset, as before.
    let other = agent_on(&h, &q, "pro").await.id;
    turn_over(&h, &other, "Bonjour").await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let o = h.agent(&other);
    assert_eq!((o.engine, o.handed_from), (EngineId::CLAUDE, None));
    assert!(o.resume_at.is_some());
    assert!(!h.items(&other).iter().any(|i| i["kind"] == "handover"));
}

#[tokio::test]
async fn an_account_is_usable_for_a_handover_unless_out_for_its_quota_by_the_pace_of_its_windows() {
    // PR #20: the same judgement as the choice of the account (`pace`): a week on pace is usable
    // past the threshold, one behind too (only its pace is bad), one at its limit is spent.
    use crate::model::{now_ms, RateWindow};
    use crate::usage::Reading;
    let h = harness("handover-usable");
    let now = now_ms();
    let window = |pct: f64, minutes: i64| {
        Some(RateWindow {
            pct,
            resets_at: Some(now + minutes * 60_000),
        })
    };
    let read = |five, week| {
        h.core
            .record_usage("principal", Reading::claude((five, week)))
    };
    h.set_settings(|s| s.quota_pause = 100);
    assert!(h.core.account_usable("principal"), "nothing read");
    // 97 % of the week, starting again in 3 h: 98.8 % at the reset.
    read(None, window(97.0, 180));
    assert!(h.core.account_usable("principal"));
    // 60 % at its second day: 210 % at the reset, behind but usable.
    read(None, window(60.0, 5 * 24 * 60));
    assert!(h.core.account_usable("principal"));
    // At its limit: spent, for the week as for the 5 hours.
    read(None, window(100.0, 180));
    assert!(!h.core.account_usable("principal"));
    read(window(100.0, 60), window(20.0, 5 * 24 * 60));
    assert!(!h.core.account_usable("principal"));
    // Under « Pause au-delà du quota », the autopilot's own ceiling stays what it was: a week on
    // pace (and under the 95 % margin, `pace::NEAR_LIMIT`) is judged good, and still held back past
    // 90 % (PR #20 leaves the pause alone).
    h.set_settings(|s| s.quota_pause = 90);
    read(window(10.0, 60), window(93.0, 180));
    assert!(!h.core.account_usable("principal"));
    assert_eq!(
        crate::accounts::standing(&h.core.usage.lock().accounts, "principal", 90, now),
        crate::pace::Standing::Good
    );
}

#[tokio::test]
async fn a_private_projects_agent_is_handed_over_by_the_option_only_to_an_account_of_its_own() {
    let h = harness("handover-private");
    let (p, _) = two_projects(&h).await;
    h.core
        .save_settings({
            let mut s = h.core.settings.read().clone();
            s.handover_on_limit = true;
            // Relais, Minimal's account, serves every project now.
            s.accounts
                .iter_mut()
                .find(|a| a.id == "relais")
                .unwrap()
                .projects = vec![];
            s
        })
        .unwrap();
    let mut project = h.core.project(&p.id).unwrap();
    project.private = true;
    h.core.update_project(project).unwrap();
    // P is private, Relais shared: the agent waits for the reset, on Claude Code.
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let a = h.agent(&id);
    assert_eq!((a.engine, a.handed_from), (EngineId::CLAUDE, None));
    assert!(a.resume_at.is_some());
    // By hand, the user still may (« Continuer sur Minimal »).
    assert_eq!(h.core.handover_targets(&id).unwrap(), [EngineId::BENCH]);
    // Relais P's own again: the next one goes on on it by itself.
    h.core
        .save_settings({
            let mut s = h.core.settings.read().clone();
            s.accounts
                .iter_mut()
                .find(|a| a.id == "relais")
                .unwrap()
                .projects = vec![p.id.clone()];
            s
        })
        .unwrap();
    let next = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &next, "Bonjour").await;
    agent_when(&h, &next, "the handover", |m| m.engine == EngineId::BENCH).await;
}

#[tokio::test]
async fn a_ticket_agent_handed_over_by_the_option_keeps_its_ticket_going() {
    let h = harness("handover-ticket");
    let (p, _) = two_projects(&h).await;
    h.core
        .save_settings({
            let mut s = h.core.settings.read().clone();
            s.handover_on_limit = true;
            s
        })
        .unwrap();
    // The ticket starts on Pro, Claude Code's only account for P.
    let mut project = h.core.project(&p.id).unwrap();
    project.account = "pro".into();
    h.core.update_project(project).unwrap();
    h.set_board(&p.id, |s| s.autopilot = false);
    let ticket = h
        .core
        .ticket_create(&p.id, crate::tickets_tests::draft("Passation", &[], 5))
        .await
        .unwrap();
    h.core.ticket_start(&ticket.id).unwrap();
    h.wait("its agent", |h| {
        h.core.ticket(&ticket.id).unwrap().agent_id.is_some()
    })
    .await;
    let t = h.core.ticket(&ticket.id).unwrap();
    let id = t.agent_id.clone().unwrap();
    let m = agent_when(&h, &id, "the handover", |m| m.engine == EngineId::BENCH).await;
    assert_eq!(m.ticket_id.as_deref(), Some(ticket.id.as_str()));
    h.wait("Minimal's reply", |h| {
        h.turns(&id).last().is_some_and(|t| t["engine"] == "bench")
    })
    .await;
    let file = h.dossier(&id).unwrap();
    assert!(
        file.contains(&format!("ticket {} — Passation", t.key)),
        "{file}"
    );
    // Its ticket goes on with it, not blocked by the limit: the board reads the turn Minimal
    // ran (which gives no report of the criteria, Minimal knowing no protocol).
    h.wait("the board read Minimal's turn", |h| {
        h.core.ticket(&ticket.id).unwrap().blocked.is_some()
    })
    .await;
    let t = h.core.ticket(&ticket.id).unwrap();
    assert_eq!(t.blocked.as_deref(), Some("Bilan des critères manquant"));
    assert_eq!(t.column, crate::model::Column::Doing);
    assert_eq!(t.agent_id.as_deref(), Some(id.as_str()));
}
