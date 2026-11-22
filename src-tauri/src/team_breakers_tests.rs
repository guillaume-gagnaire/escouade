//! The circuit breaker in the core (TS-15): through the locks, scoped to the team's space.

use super::*;
use crate::locks::{Locks, Managed, HARNESSES, MODELS, PLUGINS_DENIED};
use escouade_team::breaker::Scope;

fn breaker(what: What, scope: Scope) -> Breaker {
    Breaker {
        id: "aa".repeat(16),
        scope,
        what,
        reason: "fuite de clé chez le fournisseur X".into(),
        by: "Claire Martin".into(),
        member: "01".repeat(16),
        at: 1,
        serial: 1,
        seq: 1,
    }
}

fn team_locks(breakers: &[Breaker]) -> Locks {
    let mut l = Locks::compute(crate::locks::KEYS, None, &Managed::default());
    l.apply_breakers("Acme", breakers);
    l
}

#[test]
fn a_breaker_is_a_restriction_of_the_locks_that_names_who_set_it() {
    let l = team_locks(&[
        breaker(What::Harness { id: "codex".into() }, Scope::Team),
        breaker(
            What::Model {
                id: "claude-opus-5-5".into(),
            },
            Scope::Team,
        ),
        breaker(
            What::Plugin {
                id: "acme.rules".into(),
                uninstall: false,
            },
            Scope::Team,
        ),
    ]);
    let by = LockedBy::Breaker {
        team: "Acme".into(),
        by: "Claire Martin".into(),
        reason: "fuite de clé chez le fournisseur X".into(),
    };
    // The harness: denied, said with who and why; another passes.
    assert_eq!(l.get(HARNESSES).unwrap().denied_by("codex"), Some(&by));
    let why = l.check_engine("codex").unwrap_err().to_string();
    assert!(
        why.contains("codex") && why.contains("Claire Martin") && why.contains("fournisseur X"),
        "{why}"
    );
    assert!(l.check_engine("claude").is_ok());
    // The model, behind a provider too.
    assert!(l.check_model("claude", "claude-opus-5-5").is_err());
    assert!(l
        .check_model("opencode", "anthropic/claude-opus-5-5")
        .is_err());
    assert!(l.check_model("claude", "claude-sonnet-5-5").is_ok());
    assert!(l.get(MODELS).is_some());
    // The plugin: refused at its install.
    assert_eq!(l.plugin_cut("acme.rules"), Some(&by));
    assert!(l.check_plugin("acme.rules", "store").is_err());
    assert!(l.check_plugin("acme.other", "store").is_ok());
    assert!(l.get(PLUGINS_DENIED).is_some());
    // Nothing else is locked: a breaker only restricts.
    assert!(l.halt.is_none());
    assert!(l.rows.is_empty(), "no row of the settings greyed");
}

#[test]
fn every_agent_stopped_refuses_every_start() {
    let l = team_locks(&[breaker(What::Agents, Scope::Team)]);
    let why = l.check_engine("claude").unwrap_err().to_string();
    assert!(why.contains("arrêtés") || why.contains("stopped"), "{why}");
    assert!(asked_by(l.halt.as_ref().unwrap()).contains("Claire Martin"));
}

#[test]
fn a_breaker_of_a_project_holds_for_that_project_alone() {
    let api = "ab".repeat(16);
    let l = team_locks(&[breaker(
        What::Harness { id: "codex".into() },
        Scope::Project {
            project: api.clone(),
            name: "API".into(),
        },
    )]);
    assert!(l.check_engine("codex").is_ok(), "the team's other projects");
    assert!(l.for_project(Some(&api)).check_engine("codex").is_err());
    assert!(l
        .for_project(Some(&"cd".repeat(16)))
        .check_engine("codex")
        .is_ok());
    assert!(l.for_project(None).check_engine("codex").is_ok());
}

#[test]
fn no_new_team_agent_without_contact_for_n_hours() {
    let now = 1_800_000_000;
    assert_eq!(contact_refusal("Acme", 24, Some(now - 3600), now), None);
    assert_eq!(
        contact_refusal("Acme", 24, Some(now - 24 * 3600), now),
        None
    );
    let late = contact_refusal("Acme", 24, Some(now - 25 * 3600), now).unwrap();
    assert!(late.contains("Acme") && late.contains("24"), "{late}");
    assert!(contact_refusal("Acme", 1, None, now).is_some());
}

// ---------- in one core, the breakers given as the team's reading gives them ----------

fn view(breakers: Vec<Breaker>) -> View {
    View {
        active: breakers,
        ..Default::default()
    }
}

/// A plugin taken out: disabled with who and why, the user's switch untouched, enabled again once lifted; uninstalled
/// when the breaker says so.
#[tokio::test]
async fn a_plugin_taken_out_is_disabled_then_uninstalled_when_said() {
    let h = crate::core_tests::harness("tbrk-plugin");
    let src = h.dir.join("plugin-src");
    std::fs::create_dir_all(src.join("themes")).unwrap();
    std::fs::write(
        src.join(escouade_plugins::manifest::FILE),
        serde_json::json!({
            "id": "dev.exemple.contraste",
            "name": "Contraste",
            "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.1.0" },
            "contributes": { "themes": [{ "id": "contrast" }] }
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(src.join("themes/dark.json"), "{}").unwrap();
    let lang = crate::i18n::Lang::Fr;
    let installed = h.core.plugins.install_folder(&src, lang).unwrap();
    assert_eq!(installed.state, crate::plugins::State::Ready);
    let id = installed.id.clone();
    let cut = |uninstall| {
        breaker(
            What::Plugin {
                id: id.clone(),
                uninstall,
            },
            Scope::Team,
        )
    };
    h.core
        .set_team_breakers_of("acme", "Acme", Some(&view(vec![cut(false)])));
    let v = h.core.plugins.get(&id, lang).unwrap();
    match &v.state {
        crate::plugins::State::Disabled { reason } => assert!(
            reason.starts_with("Arrêt demandé par Claire Martin : fuite de clé"),
            "{reason}"
        ),
        other => panic!("{other:?}"),
    }
    assert!(v.choices.enabled, "the user's switch is not written");
    // Its install again is refused, said.
    assert!(h
        .core
        .locks
        .read()
        .check_plugin(&id, "folder")
        .unwrap_err()
        .to_string()
        .contains("Claire Martin"));
    // Lifted: enabled again, as it was.
    h.core
        .set_team_breakers_of("acme", "Acme", Some(&view(vec![])));
    assert_eq!(
        h.core.plugins.get(&id, lang).unwrap().state,
        crate::plugins::State::Ready
    );
    // Uninstalled when it says so.
    h.core
        .set_team_breakers_of("acme", "Acme", Some(&view(vec![cut(true)])));
    assert!(h.core.plugins.get(&id, lang).is_none(), "uninstalled");
}

/// A harness taken out while an agent of the team is at work: its turn is not cut, it stops at the end of it, said;
/// a personal project's agent of the same harness is never touched.
#[tokio::test]
async fn a_harness_taken_out_stops_an_agent_at_work_between_two_turns() {
    use crate::core::AgentOptions;
    use crate::engine::minimal::ONE_BYPASS;
    use crate::model::AgentStatus;
    let h = crate::core_tests::harness("tbrk-turn");
    crate::modes::tests::allow_bypass(&h, true);
    let engine = crate::modes::tests::with_account(&h, &ONE_BYPASS);
    let (p, _) = h.project(false).await;
    h.core.set_project_team(&p.id, Some("acme".into())).unwrap();
    let a = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(engine),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    h.wait("its process started ahead", |h| h.alive(&a.id))
        .await;
    h.core.agent(&a.id).unwrap().lock().meta.status = AgentStatus::Running;
    h.core.set_team_breakers_of(
        "acme",
        "Acme",
        Some(&view(vec![breaker(
            What::Harness {
                id: engine.as_str().into(),
            },
            Scope::Team,
        )])),
    );
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(h.alive(&a.id), "its turn is not cut");
    h.core.agent(&a.id).unwrap().lock().meta.status = AgentStatus::Idle;
    h.core.settle_locked_mode(&a.id).await;
    assert!(!h.alive(&a.id));
    assert!(h
        .items(&a.id)
        .iter()
        .filter(|i| i["kind"] == "notice")
        .any(|i| i["text"]
            .as_str()
            .is_some_and(|t| t.contains("Claire Martin") && t.contains("à la fin de son tour"))));
}

/// The team's revocation list (TS-14), read by the circuit breaker: a plugin it withdraws in every version is disabled
/// through the locks and refused at its install, said with the list's reason; a withdrawal of some versions stays the
/// list's (`Plugins::apply_team`); the list taken back gives the plugin back as it was.
#[tokio::test]
async fn the_circuit_breaker_reads_the_teams_revocation_list() {
    use crate::plugins::store::{Reason, RevocationEntry, RevocationList};
    let h = crate::core_tests::harness("tbrk-withdrawn");
    let src = h.dir.join("plugin-src");
    std::fs::create_dir_all(src.join("themes")).unwrap();
    std::fs::write(
        src.join(escouade_plugins::manifest::FILE),
        serde_json::json!({
            "id": "fr.acme.rules",
            "name": "Règles",
            "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.1.0" },
            "contributes": { "themes": [{ "id": "contrast" }] }
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(src.join("themes/dark.json"), "{}").unwrap();
    let lang = crate::i18n::Lang::Fr;
    let id = h.core.plugins.install_folder(&src, lang).unwrap().id;
    let entry = |versions: serde_json::Value| RevocationEntry {
        id: id.clone(),
        versions,
        action: "disable".into(),
        reason: Reason {
            fr: "fuite d’un jeton".into(),
            en: "a token leak".into(),
        },
        date: "2026-11-22".into(),
    };
    let list = |entries| RevocationList {
        serial: 1,
        issued_at: "2026-11-22T00:00:00Z".into(),
        entries,
        keys_revoked: None,
    };
    // Some versions only: the list's own, no lock.
    h.core.set_team_withdrawals(
        "acme",
        Some(&list(vec![entry(serde_json::json!(["0.9.0"]))])),
    );
    assert!(h.core.locks.read().plugin_cut(&id).is_none());
    assert!(h.core.team_withdrawn("acme").is_empty());
    // Every version: a lock that names the list, the plugin disabled, its install refused.
    h.core
        .set_team_withdrawals("acme", Some(&list(vec![entry(serde_json::json!("*"))])));
    assert!(matches!(
        h.core.locks.read().plugin_cut(&id),
        Some(LockedBy::Withdrawn { team, reason }) if team == "acme" && reason == "fuite d’un jeton"
    ));
    match &h.core.plugins.get(&id, lang).unwrap().state {
        crate::plugins::State::Disabled { reason } => assert!(
            reason.starts_with(
                "Retiré par la liste de révocation de l’équipe acme : fuite d’un jeton"
            ),
            "{reason}"
        ),
        other => panic!("{other:?}"),
    }
    let refused = h
        .core
        .locks
        .read()
        .check_plugin(&id, "team-store")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("liste de révocation"), "{refused}");
    assert_eq!(h.core.team_withdrawn("acme").len(), 1);
    // The list takes it back: the plugin as it was.
    h.core.set_team_withdrawals("acme", Some(&list(vec![])));
    assert!(h.core.locks.read().plugin_cut(&id).is_none());
    assert_eq!(
        h.core.plugins.get(&id, lang).unwrap().state,
        crate::plugins::State::Ready
    );
}
