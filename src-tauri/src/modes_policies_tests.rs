//! The policies judge the mode a harness really runs in (ESC-497), on every path. ESC-489 made the mode judged the one
//! the harness runs in; the policies of the device (rank 6) and of a project's team (rank 5) — `permissions.allowBypass`,
//! `permissions.modes`, `permissions.defaultMode` — judge that same one, never the mode the agent was asked to be in.
//! The fakes are ESC-489's: « Sans Filet » (`ONE_BYPASS`, Bypass alone) and « Sans Question » (`NO_ASK`: Plan, Édits
//! auto, Bypass; no « Demander », no Auto). « Autoriser le mode Bypass » is on in the app wherever a policy turns it off:
//! the policy alone holds.

use super::*;
use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::engine::minimal::{NO_ASK, ONE_BYPASS};
use crate::locks::{Managed, TeamPolicies};
use crate::model::*;
use crate::modes::tests::{allow_bypass, with_account};
use crate::tickets_tests::draft;
use serde_json::{json, Value};

/// Who imposes: this device's policy, or the team of the project.
#[derive(Clone, Copy, Debug, PartialEq)]
enum By {
    Device,
    Team,
}

const BOTH: [By; 2] = [By::Device, By::Team];

impl By {
    /// What a refusal says of who imposes.
    fn says(self) -> &'static str {
        match self {
            By::Device => "par la stratégie de ce poste",
            By::Team => "par l’équipe Acme",
        }
    }
}

/// `pairs` imposed by `by`: the device's policy, or the team « Acme » of the project `p` (its project, made one of the
/// team). The locks are made again and applied, as at a change of the policies.
fn impose(h: &Harness, p: &Project, by: By, pairs: &[(&str, Value)]) {
    let pairs = pairs.iter().map(|(k, v)| ((*k).to_owned(), v.clone()));
    match by {
        By::Device => h.core.set_managed(Managed {
            values: pairs
                .map(|(k, v)| (k, (v, "the machine's policy".to_owned())))
                .collect(),
            warnings: Vec::new(),
        }),
        By::Team => {
            h.core
                .projects
                .write()
                .iter_mut()
                .find(|x| x.id == p.id)
                .unwrap()
                .team = Some("acme".into());
            h.core.set_team_policies(Some(TeamPolicies {
                id: "acme".into(),
                team: "Acme".into(),
                signed: None,
                recommended: Default::default(),
                enforced: pairs.collect(),
            }));
        }
    }
}

/// Bypass turned off by `by`.
fn no_bypass(h: &Harness, p: &Project, by: By) {
    impose(h, p, by, &[("permissions.allowBypass", json!(false))]);
}

/// The modes allowed by `by`.
fn modes(h: &Harness, p: &Project, by: By, allowed: &[&str]) {
    impose(h, p, by, &[("permissions.modes", json!(allowed))]);
}

/// A harness, a project, and « Autoriser le mode Bypass » on in the app.
async fn setup(name: &str) -> (Harness, Project) {
    let h = harness(name);
    let (p, _) = h.project(false).await;
    allow_bypass(&h, true);
    (h, p)
}

/// A new agent of `engine` in `p`, as the window makes it (`judge`: as the Kanban and the tools do).
async fn make(
    h: &Harness,
    p: &Project,
    engine: EngineId,
    mode: Option<&str>,
    judge: bool,
) -> anyhow::Result<AgentMeta> {
    h.core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(engine),
                mode: mode.map(str::to_string),
                judge,
                select: true,
                ..Default::default()
            },
        )
        .await
        .map(|a| a.meta)
}

fn notices(h: &Harness, id: &str) -> Vec<String> {
    h.items(id)
        .into_iter()
        .filter(|i| i["kind"] == "notice")
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

async fn send(h: &Harness, id: &str) -> Result<(), String> {
    h.core
        .send_message(id, "Bonjour".into(), Vec::new())
        .await
        .map_err(|e| e.to_string())
}

/// What a refusal of Sans Filet's one mode, Bypass turned off by `by`, says.
fn sans_filet_refused(why: &str, by: By) {
    // A ticket's reason of a block says it is an error.
    let why = why.strip_prefix("Erreur : ").unwrap_or(why);
    assert!(
        why.starts_with("Sans Filet ne tourne qu’en « Bypass » : "),
        "{why}"
    );
    assert!(
        why.contains(&format!("imposé {} (permissions.allowBypass)", by.says())),
        "{why}"
    );
    assert!(why.ends_with("Choisis un autre harness."), "{why}");
}

// ---------- the mode of no question open to a project ----------

/// `bypass_allowed_in`: this device's policy turns the setting itself off; a team's is its projects' alone, which the
/// setting of the app does not hold, and it is what the launches of its agents hold (their arguments of no question).
#[tokio::test]
async fn bypass_is_open_to_a_project_only_when_the_setting_and_its_policies_allow_it() {
    let (h, p) = setup("esc497-open").await;
    assert!(h.core.bypass_allowed_in(&p.id));
    impose(
        &h,
        &p,
        By::Team,
        &[("permissions.allowBypass", json!(false))],
    );
    assert!(h.core.bypass_allowed(), "the app's setting stays");
    assert!(!h.core.bypass_allowed_in(&p.id), "its team's projects");
    // A bound of the modes without Bypass shuts it too.
    impose(
        &h,
        &p,
        By::Team,
        &[("permissions.modes", json!(["plan", "default"]))],
    );
    assert!(!h.core.bypass_allowed_in(&p.id));
    h.core.set_team_policies(None);
    assert!(h.core.bypass_allowed_in(&p.id));
    // This device's: the setting brought off (`clamp_settings`), seen by every path.
    impose(
        &h,
        &p,
        By::Device,
        &[("permissions.allowBypass", json!(false))],
    );
    assert!(!h.core.bypass_allowed());
    assert!(!h.core.bypass_allowed_in(&p.id));
}

// ---------- a new agent: the window, a tool, the Kanban ----------

/// A new agent of a harness whose one mode is Bypass, Bypass turned off by a policy: not made, by the window as by a
/// tool or the Kanban, and the refusal says the harness runs in Bypass alone and who turns it off.
#[tokio::test]
async fn a_new_agent_of_a_harness_whose_one_mode_is_bypass_is_not_made_when_a_policy_turns_bypass_off(
) {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-new-one-{by:?}")).await;
        let e = with_account(&h, &ONE_BYPASS);
        no_bypass(&h, &p, by);
        for judge in [false, true] {
            for asked in [None, Some("default"), Some("acceptEdits")] {
                let why = make(&h, &p, e, asked, judge).await.unwrap_err().to_string();
                sans_filet_refused(&why, by);
            }
        }
        assert!(h.core.project_agents(&p.id).is_empty(), "{by:?}");
    }
}

/// A new agent of a harness of several modes without « Demander »: asked « Demander », it would run in « Plan »; a
/// bound of the modes without « Plan » refuses it, saying so. Its account's mode goes to the nearest one of its harness
/// the bound allows, never one that allows more; none: refused.
#[tokio::test]
async fn a_new_agent_of_a_harness_without_ask_is_judged_in_the_mode_it_runs_in_by_the_bound_of_the_modes(
) {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-new-noask-{by:?}")).await;
        let e = with_account(&h, &NO_ASK);
        modes(&h, &p, by, &["default", "acceptEdits", "auto"]);
        for judge in [false, true] {
            let why = make(&h, &p, e, Some("default"), judge)
                .await
                .unwrap_err()
                .to_string();
            assert_eq!(
                why,
                format!("Sans Question n’a pas le mode « Demander » et tournerait en « Plan » : Le mode « plan » n’est pas permis {}. Choisis un autre mode ou un autre harness.", by.says()),
                "{by:?}"
            );
        }
        // Its account's (« Édits auto », the mode of the first start as it runs it) is allowed: kept.
        let a = make(&h, &p, e, None, false).await.unwrap();
        assert_eq!(a.mode, "acceptEdits");
        // The bound « Plan », « Demander »: its account's goes to « Demander », which it would run as « Plan ».
        modes(&h, &p, by, &["plan", "default"]);
        let b = make(&h, &p, e, None, true).await.unwrap();
        assert_eq!(b.mode, "plan", "{by:?}");
        // The bound « Demander », « Auto »: none of its modes allows as little and is allowed.
        modes(&h, &p, by, &["default", "auto"]);
        assert!(make(&h, &p, e, None, true).await.is_err(), "{by:?}");
    }
}

/// The mode by default imposed (`permissions.defaultMode`), one its harness does not have: a new agent goes to the one
/// its harness runs it in, never one that allows more; a team's holds for its projects, where the app's setting does
/// not have it.
#[tokio::test]
async fn the_mode_by_default_imposed_that_its_harness_lacks_is_the_closest_that_allows_no_more() {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-default-{by:?}")).await;
        let e = with_account(&h, &NO_ASK);
        let one = with_account(&h, &ONE_BYPASS);
        impose(&h, &p, by, &[("permissions.defaultMode", json!("default"))]);
        // Escouade's own harness: the mode imposed, the app's own setting set to « Auto ».
        let c = h.core.create_agent(&p.id, None).await.unwrap().meta;
        assert_eq!(c.mode, "default", "{by:?}");
        let a = make(&h, &p, e, None, true).await.unwrap();
        assert_eq!(a.mode, "plan", "{by:?}");
        // The harness of Bypass alone, the bound without Bypass: refused, never made.
        impose(
            &h,
            &p,
            by,
            &[
                ("permissions.defaultMode", json!("default")),
                ("permissions.modes", json!(["plan", "default"])),
            ],
        );
        let why = make(&h, &p, one, None, false)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            why.starts_with("Sans Filet ne tourne qu’en « Bypass » : Le mode « bypassPermissions » n’est pas permis"),
            "{why}"
        );
        assert!(why.contains(by.says()), "{why}");
    }
}

// ---------- a start: the first, a resume, the view ----------

/// What turns Bypass off while an agent runs in it: a policy (of this device, of the project's team), or « Autoriser le
/// mode Bypass » turned off in the app.
#[derive(Clone, Copy, Debug)]
enum Off {
    Policy(By),
    Setting,
}

impl Off {
    const ALL: [Off; 3] = [Off::Policy(By::Device), Off::Policy(By::Team), Off::Setting];

    /// Bypass turned off, while the app runs.
    fn apply(self, h: &Harness, p: &Project) {
        match self {
            Off::Policy(by) => no_bypass(h, p, by),
            Off::Setting => allow_bypass(h, false),
        }
    }

    /// Whether a refusal says why, as its cause says it.
    fn said_in(self, why: &str) -> bool {
        match self {
            Off::Policy(by) => why.contains(by.says()) && why.contains("(permissions.allowBypass)"),
            Off::Setting => {
                why.contains(ONLY_BYPASS_FR) && why.contains("Autoriser le mode Bypass")
            }
        }
    }
}

const ONLY_BYPASS_FR: &str = "Sans Filet ne tourne qu’en";

/// An agent of a harness whose one mode is Bypass, made while Bypass is allowed: its process starts ahead (`warm`), in
/// Bypass. Bypass turned off while it is at rest (a policy, of the device or of its team, or the app's setting): that
/// process stops at once, its conversation says why, and nothing starts it again — its message, « Relancer », its view,
/// the planned resume, a warm-up — each refused with the same reason.
#[tokio::test]
async fn a_process_started_ahead_in_bypass_stops_when_bypass_is_turned_off_and_nothing_starts_it_again(
) {
    for off in Off::ALL {
        let (h, p) = setup(&format!("esc497-ahead-{off:?}")).await;
        let one = with_account(&h, &ONE_BYPASS);
        let a = make(&h, &p, one, Some("default"), false).await.unwrap();
        assert_eq!(a.mode, BYPASS);
        // Its warm-up: up, in Bypass (allowed then).
        h.wait("its process started ahead", |h| h.alive(&a.id))
            .await;
        off.apply(&h, &p);
        h.wait("its process stopped", |h| !h.alive(&a.id)).await;
        h.wait("its notice", |h| {
            notices(h, &a.id)
                .iter()
                .any(|n| off.said_in(n) && n.contains("L’agent s’arrête"))
        })
        .await;
        // Its message.
        let why = send(&h, &a.id).await.unwrap_err();
        assert!(off.said_in(&why), "{off:?}: {why}");
        assert!(!h.alive(&a.id));
        // « Relancer ».
        let why = h.core.restart_process(&a.id).await.unwrap_err().to_string();
        assert!(off.said_in(&why), "{off:?}: {why}");
        assert!(!h.alive(&a.id));
        // Its view (« Vue terminal »).
        let why = h
            .core
            .open_agent_terminal(&a.id, (80, 24), true, |_| {})
            .await
            .unwrap_err()
            .to_string();
        assert!(off.said_in(&why), "{off:?}: {why}");
        // The planned resume after the usage limit.
        h.core.settings.write().auto_resume = true;
        h.core.agent(&a.id).unwrap().lock().meta.resume_at = Some(1);
        h.core.resume_due().await;
        assert!(
            notices(&h, &a.id)
                .iter()
                .any(|n| n.starts_with("Reprise automatique impossible") && off.said_in(n)),
            "{off:?}: {:?}",
            notices(&h, &a.id)
        );
        // A warm-up (a new selection, a change of account…).
        assert!(h.core.ensure_process_for(&a.id, true).await.is_err());
        assert!(!h.alive(&a.id), "{off:?}");
        assert_eq!(h.agent(&a.id).mode, BYPASS, "its mode stays");
    }
}

/// The same agent at work when Bypass is turned off: its turn is not cut (« Un agent dans un mode devenu interdit finit
/// son tour »), then its process stops, said.
#[tokio::test]
async fn an_agent_at_work_when_bypass_is_turned_off_ends_its_turn_then_stops() {
    for off in Off::ALL {
        let (h, p) = setup(&format!("esc497-at-work-{off:?}")).await;
        let one = with_account(&h, &ONE_BYPASS);
        let a = make(&h, &p, one, Some("default"), false).await.unwrap();
        h.wait("its process started ahead", |h| h.alive(&a.id))
            .await;
        // At work: settled at the end of its turn, as `turn_ended` does it.
        h.core.agent(&a.id).unwrap().lock().meta.status = AgentStatus::Running;
        off.apply(&h, &p);
        assert!(h.alive(&a.id), "{off:?}: not cut during its turn");
        h.core.agent(&a.id).unwrap().lock().meta.status = AgentStatus::Idle;
        h.core.settle_locked_mode(&a.id).await;
        assert!(!h.alive(&a.id), "{off:?}");
        assert!(
            notices(&h, &a.id)
                .iter()
                .any(|n| off.said_in(n) && n.contains("L’agent s’arrête à la fin de son tour")),
            "{off:?}: {:?}",
            notices(&h, &a.id)
        );
    }
}

/// A turn its harness began by itself (the ACP pilot's, ESC-491: Unharnessed once a command of its last cell ended)
/// is at work like any other: Bypass turned off during it, its process is not cut; its end, told as any other's
/// (`turn_ended`), stops it, said (ESC-520).
#[tokio::test]
async fn a_turn_its_harness_began_by_itself_ends_then_its_process_stops_when_bypass_was_turned_off()
{
    for off in Off::ALL {
        let (h, p) = setup(&format!("esc520-own-turn-{off:?}")).await;
        let one = with_account(&h, &ONE_BYPASS);
        let a = make(&h, &p, one, Some("default"), false).await.unwrap();
        h.wait("its process started ahead", |h| h.alive(&a.id))
            .await;
        // The harness begins a turn, no message of Escouade's under way.
        h.core.agent(&a.id).unwrap().lock().meta.status = AgentStatus::Running;
        off.apply(&h, &p);
        assert!(h.alive(&a.id), "{off:?}: not cut during its own turn");
        h.core.agent(&a.id).unwrap().lock().meta.status = AgentStatus::Done;
        h.core
            .turn_ended(
                &a.id,
                crate::board::TurnEnd::Finished("Réveillé par la fin de sa commande.".into()),
            )
            .await;
        assert!(!h.alive(&a.id), "{off:?}");
        assert!(
            notices(&h, &a.id)
                .iter()
                .any(|n| off.said_in(n) && n.contains("L’agent s’arrête à la fin de son tour")),
            "{off:?}: {:?}",
            notices(&h, &a.id)
        );
    }
}

/// An agent made while its mode was allowed, then a policy that does not allow the mode its harness runs it in: a
/// harness without « Demander » in « Plan », that a bound of the modes does not allow, goes to the one of its own modes
/// allowed that allows the least (« Demander », which `nearest_mode` would give, is not one), then starts there.
#[tokio::test]
async fn an_agent_whose_harness_mode_a_bound_refuses_goes_to_one_of_its_own_then_starts() {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-start-{by:?}")).await;
        let several = with_account(&h, &NO_ASK);
        let b = make(&h, &p, several, Some("default"), false).await.unwrap();
        assert_eq!(b.mode, "plan");
        h.wait("its process started ahead", |h| h.alive(&b.id))
            .await;
        modes(&h, &p, by, &["default", "acceptEdits"]);
        h.wait("its mode settled", |h| h.agent(&b.id).mode == "acceptEdits")
            .await;
        send(&h, &b.id).await.unwrap();
        h.wait("its answer", |h| {
            h.items(&b.id).iter().any(|i| i["kind"] == "text")
        })
        .await;
        assert_eq!(h.agent(&b.id).mode, "acceptEdits");
    }
}

/// An agent of the app's own harness in « Auto »: Escouade's harnesses are judged as before (the mode they run in is
/// their own), and a bound that allows it lets it start.
#[tokio::test]
async fn an_agent_of_escouade_s_own_harness_starts_as_before_within_the_policies() {
    let (h, p) = setup("esc497-own").await;
    impose(
        &h,
        &p,
        By::Device,
        &[(
            "permissions.modes",
            json!(["default", "acceptEdits", "auto"]),
        )],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert_eq!(a.mode, "auto");
    let refused = h
        .core
        .set_agent_options(&a.id, None, None, Some("plan".into()))
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(
        refused,
        "Le mode « plan » n’est pas permis par la stratégie de ce poste."
    );
}

// ---------- a change of mode ----------

/// A change of mode: one of its harness's, judged by the policies (Bypass turned off, a bound of the modes) as by
/// « Autoriser le mode Bypass ».
#[tokio::test]
async fn a_change_of_mode_is_judged_by_the_policies() {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-change-{by:?}")).await;
        let e = with_account(&h, &NO_ASK);
        let a = make(&h, &p, e, None, false).await.unwrap();
        assert_eq!(a.mode, "acceptEdits");
        no_bypass(&h, &p, by);
        let set = |mode: &'static str| {
            let h = &h;
            let id = a.id.clone();
            async move {
                h.core
                    .set_agent_options(&id, None, None, Some(mode.into()))
                    .await
                    .map_err(|e| e.to_string())
            }
        };
        let why = set(BYPASS).await.unwrap_err();
        assert!(why.contains(by.says()), "{why}");
        // Still a mode its harness does not have is refused first, as before.
        assert_eq!(
            set("default").await.unwrap_err(),
            "Sans Question n’a pas le mode « Demander » : choisis un de ses modes."
        );
        modes(&h, &p, by, &["default", "acceptEdits"]);
        assert_eq!(
            set("plan").await.unwrap_err(),
            format!("Le mode « plan » n’est pas permis {}.", by.says())
        );
        set("acceptEdits").await.unwrap();
    }
}

// ---------- the Kanban: « Lancer », the autopilot, a scheduled task ----------

/// A ticket on a harness whose one mode is Bypass, Bypass turned off by a policy, and one on a harness without
/// « Demander » whose mode the bound does not allow: blocked with why, by the autopilot and by « Lancer », no agent made.
#[tokio::test]
async fn a_ticket_is_blocked_when_the_policies_do_not_allow_the_mode_its_harness_runs_in() {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-ticket-{by:?}")).await;
        let one = with_account(&h, &ONE_BYPASS);
        let several = with_account(&h, &NO_ASK);
        h.set_board(&p.id, |s| {
            s.autopilot = false;
            s.mode = "default".into();
        });
        no_bypass(&h, &p, by);
        let ticket = |title: &str, e: EngineId| {
            let mut d = draft(title, &[], 5);
            d.engine = Some(Some(e));
            d
        };
        // By the autopilot.
        let t = h
            .core
            .ticket_create(&p.id, ticket("Pilote", one))
            .await
            .unwrap();
        h.set_board(&p.id, |s| s.autopilot = true);
        h.wait_ticket(&t.id, "blocked", |t| t.blocked.is_some())
            .await;
        let blocked = h.ticket(&t.id);
        assert!(blocked.agent_id.is_none(), "{blocked:?}");
        sans_filet_refused(blocked.blocked.as_deref().unwrap(), by);
        h.set_board(&p.id, |s| s.autopilot = false);
        // By « Lancer »: the board's « Demander », run as « Plan », that the bound does not allow.
        impose(
            &h,
            &p,
            by,
            &[("permissions.modes", json!(["default", "acceptEdits"]))],
        );
        let u = h
            .core
            .ticket_create(&p.id, ticket("À la main", several))
            .await
            .unwrap();
        h.core.ticket_start(&u.id).unwrap();
        h.wait_ticket(&u.id, "blocked", |t| t.blocked.is_some())
            .await;
        let why = h.ticket(&u.id).blocked.unwrap();
        assert!(why.contains("tournerait en « Plan »"), "{why}");
        assert!(why.contains(by.says()), "{why}");
        assert!(h.core.project_agents(&p.id).is_empty());
    }
}

/// A scheduled task (its ticket made and launched at its due time) on a harness whose one mode is Bypass, turned off
/// by a policy: its ticket blocked with why.
#[tokio::test]
async fn a_scheduled_task_on_a_harness_of_bypass_alone_is_blocked_by_the_policy() {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-scheduled-{by:?}")).await;
        let one = with_account(&h, &ONE_BYPASS);
        h.set_board(&p.id, |s| s.autopilot = false);
        no_bypass(&h, &p, by);
        let now = now_ms() + 3_600_000;
        let t0 = now - now.rem_euclid(60_000) + 30_000;
        h.core.schedule_clock.set(t0);
        let task = h
            .core
            .scheduled_create(
                &p.id,
                crate::scheduled::ScheduledDraft {
                    title: Some("Revue".into()),
                    description: Some("Relis le code.".into()),
                    engine: Some(Some(one)),
                    rhythm: Some(Rhythm {
                        kind: RhythmKind::Cron,
                        cron: "* * * * *".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        h.core.schedule_clock.set(t0 + 60_000);
        let made = h.core.scheduled_tick().await;
        assert_eq!(made.len(), 1, "{task:?}");
        h.wait_ticket(&made[0], "blocked", |t| t.blocked.is_some())
            .await;
        // Its mode the board's, empty: its account's, Bypass, the one it runs in (said as the policy says it).
        let why = h.ticket(&made[0]).blocked.unwrap();
        assert!(
            why.contains(&format!("imposé {} (permissions.allowBypass)", by.says())),
            "{why}"
        );
    }
}

// ---------- a handover ----------

/// A handover and « Continuer sur »: a harness the policies do not allow is refused, and so is the mode the target runs
/// the agent in when they do not allow it; refused before anything stops.
#[tokio::test]
async fn a_handover_is_refused_when_the_policies_do_not_allow_the_mode_the_other_harness_runs_in() {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-handover-{by:?}")).await;
        let several = with_account(&h, &NO_ASK);
        let mut s = h.core.settings.read().clone();
        s.engine_order = vec![EngineId::CLAUDE, several];
        h.core.save_settings(s).unwrap();
        let a = make(&h, &p, EngineId::CLAUDE, Some("default"), false)
            .await
            .unwrap();
        // « Demander » there is « Plan », which the bound does not allow.
        modes(&h, &p, by, &["default", "acceptEdits", "auto"]);
        let why = h
            .core
            .continue_on_engine(&a.id, several)
            .await
            .unwrap_err()
            .to_string();
        assert!(why.contains("tournerait en « Plan »"), "{why}");
        assert!(why.contains(by.says()), "{why}");
        assert_eq!(h.agent(&a.id).engine, EngineId::CLAUDE);
        // A harness not allowed.
        impose(
            &h,
            &p,
            by,
            &[("harnesses.allowed", json!([EngineId::CLAUDE.as_str()]))],
        );
        let why = h
            .core
            .continue_on_engine(&a.id, several)
            .await
            .unwrap_err()
            .to_string();
        assert!(why.contains("n’est pas permis"), "{why}");
        assert_eq!(h.agent(&a.id).engine, EngineId::CLAUDE);
    }
}

// ---------- the Overseer ----------

/// The Overseer on a harness whose one mode is Bypass, Bypass turned off by this device's policy: it does not start,
/// and its answer says why (it is of no team: the device's policies are its).
#[tokio::test]
async fn the_overseer_does_not_start_on_a_harness_of_bypass_alone_turned_off_by_the_policy() {
    let (h, p) = setup("esc497-overseer").await;
    let one = with_account(&h, &ONE_BYPASS);
    {
        let mut s = h.core.settings.write();
        s.overseer.enabled = true;
        s.mcp_enabled = true;
        s.overseer.engine = one;
    }
    no_bypass(&h, &p, By::Device);
    let why = h
        .core
        .overseer_send("Bonjour", false)
        .await
        .unwrap_err()
        .to_string();
    sans_filet_refused(&why, By::Device);
}

// ---------- an agent whose mode becomes refused ----------

/// A bound of the modes that comes while an agent is in one it no longer allows: it goes to the nearest one its harness
/// has (never a mode its harness lacks, which `nearest_mode` alone would give), said; a harness none of whose modes is
/// allowed stops between two turns.
#[tokio::test]
async fn an_agent_whose_mode_becomes_refused_goes_to_the_nearest_of_its_harness_s_modes() {
    for by in BOTH {
        let (h, p) = setup(&format!("esc497-settle-{by:?}")).await;
        let several = with_account(&h, &NO_ASK);
        let one = with_account(&h, &ONE_BYPASS);
        let a = make(&h, &p, several, None, false).await.unwrap();
        assert_eq!(a.mode, "acceptEdits");
        let b = make(&h, &p, one, None, false).await.unwrap();
        send(&h, &b.id).await.unwrap();
        h.wait("its answer", |h| {
            h.items(&b.id).iter().any(|i| i["kind"] == "text")
        })
        .await;
        h.wait("its turn over", |h| !h.agent(&b.id).status.is_active())
            .await;
        // « Plan », « Demander »: `nearest_mode` would say « Demander », which Sans Question does not have.
        modes(&h, &p, by, &["plan", "default"]);
        h.wait("the agent moved", |h| h.agent(&a.id).mode == "plan")
            .await;
        h.wait("its notice", |h| {
            notices(h, &a.id)
                .iter()
                .any(|n| n.contains("L’agent passe en mode « plan »"))
        })
        .await;
        // Sans Filet has no other mode: stopped, said.
        h.wait("Sans Filet stopped", |h| !h.alive(&b.id)).await;
        h.wait("its notice", |h| {
            notices(h, &b.id)
                .iter()
                .any(|n| n.contains("Sans Filet n’a aucun autre mode permis"))
        })
        .await;
        assert!(send(&h, &b.id).await.is_err());
    }
}
