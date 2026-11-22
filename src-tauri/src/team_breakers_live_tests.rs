//! The circuit breaker in the core against the real `escouade-team-server` (TS-15): two data folders of one machine,
//! Alice an administrator, Bob a member. A harness taken out stops Bob's team agent within the second (measured), never
//! his personal one; every agent stopped, at once, even at work; offline, it applies at the return before any new start;
//! « sans contact depuis N heures »; a breaker of one project holds for it alone.

use std::time::{Duration, Instant};

use crate::core::AgentOptions;
use crate::core_tests::Harness;
use crate::engine::minimal::ONE_BYPASS;
use crate::model::{AgentMeta, AgentStatus, EngineId, Project};
use crate::modes::tests::{allow_bypass, with_account};
use crate::teamdocs_tests::{pair, Side};
use crate::teams::BreakerAsk;
use escouade_team::breaker::{Scope, What};

const WHY: &str = "fuite de clé chez le fournisseur X";

async fn make(h: &Harness, p: &Project, engine: EngineId) -> AgentMeta {
    h.core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(engine),
                mode: Some("default".into()),
                select: true,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
}

fn notices(h: &Harness, id: &str) -> Vec<String> {
    h.items(id)
        .into_iter()
        .filter(|i| i["kind"] == "notice")
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

/// Another project of the same data folder, of its own repository: personal (of no team).
async fn personal(h: &Harness) -> Project {
    let dir = h.dir.join("perso");
    std::fs::create_dir_all(&dir).unwrap();
    let r = crate::core_tests::repo(&dir);
    h.core
        .create_project(
            &r.to_string_lossy(),
            "perso",
            "oklch(0.72 0.12 48)",
            false,
            None,
            true,
        )
        .await
        .unwrap()
}

async fn send(h: &Harness, id: &str) -> Result<(), String> {
    h.core
        .send_message(id, "Bonjour".into(), Vec::new())
        .await
        .map_err(|e| e.to_string())
}

/// Bob's side, with a fake harness of one account, a team agent and a personal one, both up (their warm-up).
async fn agents(bob: &Side) -> (EngineId, AgentMeta, AgentMeta) {
    allow_bypass(&bob.h, true);
    let engine = with_account(&bob.h, &ONE_BYPASS);
    let perso = personal(&bob.h).await;
    let team = make(&bob.h, &bob.p, engine).await;
    let mine = make(&bob.h, &perso, engine).await;
    bob.h
        .wait("both started ahead", |h| {
            h.alive(&team.id) && h.alive(&mine.id)
        })
        .await;
    (engine, team, mine)
}

/// Bob's watch, as the app runs it (its live feed), until his device is online for Alice.
async fn watch(alice: &Side, bob: &Side, id: &str) {
    tokio::spawn(bob.teams.clone().watch(Duration::from_secs(60)));
    let deadline = Instant::now() + Duration::from_secs(20);
    while alice.teams.breaker_online(id).await.unwrap_or(0) < 1 {
        assert!(Instant::now() < deadline, "Bob's live feed never connected");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Its first reading done.
    tokio::time::sleep(Duration::from_millis(300)).await;
}

fn ask(what: What, scope: Scope) -> BreakerAsk {
    BreakerAsk {
        scope,
        what,
        reason: WHY.into(),
        by: "Claire Martin".into(),
    }
}

/// How long `alive` stays true after `from`, bounded at 5 s.
async fn stopped_within(h: &Harness, id: &str, from: Instant) -> Duration {
    while h.alive(id) {
        assert!(
            from.elapsed() < Duration::from_secs(5),
            "the team agent still runs after 5 s"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    from.elapsed()
}

/// An administrator takes a harness out: on Bob's device online, the team agent at rest stops within the second
/// (measured), says who and why, and nothing starts it again; the personal agent of the same harness runs on. A member
/// sets nothing and lifts nothing. The breaker lifted, it starts again.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_harness_taken_out_stops_the_team_agent_within_the_second_and_never_a_personal_one() {
    let (_server, alice, bob, id) = pair("tbrk-harness").await;
    let (engine, team, mine) = agents(&bob).await;
    watch(&alice, &bob, &id).await;

    alice
        .teams
        .breaker_set(
            &id,
            ask(
                What::Harness {
                    id: engine.as_str().into(),
                },
                Scope::Team,
            ),
        )
        .await
        .unwrap();
    let took = stopped_within(&bob.h, &team.id, Instant::now()).await;
    eprintln!("circuit breaker (harness) applied on the other device in {took:?}");
    assert!(
        took < Duration::from_secs(1),
        "applied in {took:?}, not within the second"
    );
    assert!(
        bob.h.alive(&mine.id),
        "a personal project's agent is never touched"
    );
    bob.h
        .wait("its notice", |h| {
            notices(h, &team.id)
                .iter()
                .any(|n| n.contains("Claire Martin") && n.contains(WHY))
        })
        .await;
    // Nothing starts it again, with the reason.
    let why = send(&bob.h, &team.id).await.unwrap_err();
    assert!(why.contains("Claire Martin") && why.contains(WHY), "{why}");
    assert!(!bob.h.alive(&team.id));
    // The lock says who imposes it.
    let locks = bob.h.core.locks_for(&bob.p.id);
    assert!(matches!(
        locks.get(crate::locks::HARNESSES).and_then(|l| l.denied_by(engine.as_str())),
        Some(crate::locks::LockedBy::Breaker { by, .. }) if by == "Claire Martin"
    ));
    assert!(bob
        .h
        .core
        .locks_for(&mine.project_id)
        .check_engine(engine.as_str())
        .is_ok());

    // A member does not set one, nor lift it.
    let view = bob.teams.breakers(&id).await.unwrap();
    assert!(!view.admin);
    assert_eq!(view.view.active.len(), 1);
    let breaker = view.view.active[0].id.clone();
    assert!(bob
        .teams
        .breaker_set(&id, ask(What::Agents, Scope::Team))
        .await
        .is_err());
    assert!(bob.teams.breaker_lift(&id, &breaker, "Bob").await.is_err());

    // Lifted: it starts again.
    alice
        .teams
        .breaker_lift(&id, &breaker, "Claire Martin")
        .await
        .unwrap();
    bob.h
        .wait("the lock lifted", |h| {
            h.core
                .locks_for(&bob.p.id)
                .check_engine(engine.as_str())
                .is_ok()
        })
        .await;
    bob.h.core.ensure_process_for(&team.id, true).await.unwrap();
    assert!(bob.h.alive(&team.id));
}

/// Every agent of the team stopped: at once, even at work (as quitting the app), the turn lost; the personal agent
/// runs on; no team agent starts until it is lifted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_agent_stopped_stops_them_at_once_even_at_work_and_never_a_personal_one() {
    let (_server, alice, bob, id) = pair("tbrk-halt").await;
    let (engine, team, mine) = agents(&bob).await;
    watch(&alice, &bob, &id).await;
    // At work.
    bob.h.core.agent(&team.id).unwrap().lock().meta.status = AgentStatus::Running;
    alice
        .teams
        .breaker_set(&id, ask(What::Agents, Scope::Team))
        .await
        .unwrap();
    let took = stopped_within(&bob.h, &team.id, Instant::now()).await;
    eprintln!("circuit breaker (every agent) applied on the other device in {took:?}");
    assert!(took < Duration::from_secs(1), "stopped in {took:?}");
    assert!(
        !bob.h.agent(&team.id).status.is_active(),
        "its turn is over"
    );
    assert!(
        bob.h.alive(&mine.id),
        "a personal project's agent is never touched"
    );
    bob.h
        .wait("its notice", |h| {
            notices(h, &team.id).iter().any(|n| {
                n.starts_with("Arrêt demandé par Claire Martin : ")
                    && n.contains("reprendra sa session")
            })
        })
        .await;
    // No team agent starts, of any harness.
    let why = send(&bob.h, &team.id).await.unwrap_err();
    assert!(why.contains("Claire Martin"), "{why}");
    let other = bob
        .h
        .core
        .create_agent_with(
            &bob.p.id,
            AgentOptions {
                engine: Some(engine),
                ..Default::default()
            },
        )
        .await;
    assert!(other.is_err(), "no new agent of the team");
    assert!(bob.h.core.locks_for(&mine.project_id).halt.is_none());
}

/// Offline, a breaker set meanwhile applies at the return, before any new start of a team agent: the start reads the
/// breakers first (`team_gate`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn offline_it_applies_at_the_return_before_any_new_start() {
    let (mut server, alice, bob, id) = pair("tbrk-offline").await;
    let (engine, team, _mine) = agents(&bob).await;
    // Bob read the team, then lost the server: his reading fails, his team is stale.
    bob.teams.sync(&id).await.unwrap();
    server.stop().await;
    let _ = bob.teams.sync(&id).await;
    assert!(bob.teams.breakers_stale(&id));
    // Offline, what he does not know of applies to nothing: his agent runs on.
    assert!(bob.h.alive(&team.id));
    server.again().await;
    alice
        .teams
        .breaker_set(
            &id,
            ask(
                What::Harness {
                    id: engine.as_str().into(),
                },
                Scope::Team,
            ),
        )
        .await
        .unwrap();
    // Back, before his watch read anything: a new start reads the breakers first, and is refused.
    assert!(bob
        .h
        .core
        .locks_for(&bob.p.id)
        .check_engine(engine.as_str())
        .is_ok());
    let other = bob
        .h
        .core
        .create_agent_with(
            &bob.p.id,
            AgentOptions {
                engine: Some(engine),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let why = bob
        .h
        .core
        .ensure_process_for(&other.meta.id, true)
        .await
        .err()
        .expect("the start is refused")
        .to_string();
    assert!(why.contains("Claire Martin") && why.contains(WHY), "{why}");
    assert!(!bob.teams.breakers_stale(&id));
    bob.h
        .wait("the agent stopped", |h| !h.alive(&team.id))
        .await;
}

/// « Pas de nouvel agent d’équipe sans contact avec le serveur depuis N heures »: a policy of this device. A team agent
/// does not start past it, said with the hour of the last contact; a personal one does; a contact again lifts it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_new_team_agent_without_contact_for_n_hours() {
    let (_server, _alice, bob, id) = pair("tbrk-contact").await;
    allow_bypass(&bob.h, true);
    let engine = with_account(&bob.h, &ONE_BYPASS);
    let perso = personal(&bob.h).await;
    bob.h.core.set_managed(crate::locks::Managed {
        values: [(
            crate::locks::MAX_OFFLINE_HOURS.to_string(),
            (serde_json::json!(2), "the machine's policy".to_string()),
        )]
        .into(),
        warnings: Vec::new(),
    });
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    bob.teams.set_last_contact(&id, Some(now - 3 * 3600));
    let team = bob
        .h
        .core
        .create_agent_with(
            &bob.p.id,
            AgentOptions {
                engine: Some(engine),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    let why = send(&bob.h, &team.id).await.unwrap_err();
    assert!(
        why.starts_with("Pas de nouvel agent de l’équipe ") && why.contains("2 dernières heures"),
        "{why}"
    );
    assert!(!bob.h.alive(&team.id));
    // A personal project's agent starts.
    let mine = make(&bob.h, &perso, engine).await;
    bob.h
        .wait("the personal agent up", |h| h.alive(&mine.id))
        .await;
    // The server answers again: the team agent starts.
    bob.teams.sync(&id).await.unwrap();
    bob.h.core.ensure_process_for(&team.id, true).await.unwrap();
    assert!(bob.h.alive(&team.id));
}

/// A breaker of one project of the team holds for it alone: the team's other projects go on.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_breaker_of_one_project_holds_for_it_alone() {
    let (_server, alice, bob, id) = pair("tbrk-project").await;
    let journal = "ab".repeat(16);
    {
        let mut projects = bob.h.core.projects.write();
        let p = projects.iter_mut().find(|x| x.id == bob.p.id).unwrap();
        p.team_project = Some(journal.clone());
    }
    let other = personal(&bob.h).await;
    bob.h
        .core
        .set_project_team(&other.id, Some(id.clone()))
        .unwrap();
    alice
        .teams
        .breaker_set(
            &id,
            ask(
                What::Model {
                    id: "gpt-5.5".into(),
                },
                Scope::Project {
                    project: journal.clone(),
                    name: "API".into(),
                },
            ),
        )
        .await
        .unwrap();
    bob.teams.sync(&id).await.unwrap();
    assert!(bob
        .h
        .core
        .locks_for(&bob.p.id)
        .check_model("codex", "gpt-5.5")
        .is_err());
    assert!(bob
        .h
        .core
        .locks_for(&other.id)
        .check_model("codex", "gpt-5.5")
        .is_ok());
}
