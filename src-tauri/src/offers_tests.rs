//! The proposals at the right moment (F-3, ESC-558): each signal at its threshold and just under it, the rules of the
//! rhythm, a policy that forbids, « Ne plus proposer » kept across a restart, and nothing that leaves the machine.

use super::*;
use crate::core_tests::harness;
use crate::locks::Managed;
use crate::paths::{test_dir, DataDir};
use serde_json::json;
use tauri::test::mock_app;

/// A moment in the afternoon, in UTC (offset 0): days and hours are easy to count.
const NOW: i64 = 1_800_000_000_000 - 1_800_000_000_000 % DAY_MS + 15 * 60 * MINUTE_MS;
const H: i64 = 60 * MINUTE_MS;

fn off(fs: &[Feature]) -> Features {
    let mut s = Features::default();
    for f in fs {
        s.set(*f, false);
    }
    s
}

fn none(_: Feature) -> bool {
    false
}

/// The proposal for the switches `features` with what `saved` and `live` noticed, at `now`, quiet.
fn offer_of(saved: &FeatureOffers, features: &Features, live: &Live, now: i64) -> Option<Offer> {
    candidate(saved, features, &none, live, Moment::default(), now, 0)
}

fn times(n: usize, every: i64) -> Vec<i64> {
    (0..n as i64).map(|i| NOW - i * every).collect()
}

#[test]
fn kanban_off_two_tickets_refused_by_the_mcp_server_in_a_week_propose_it_one_does_not() {
    let kanban = off(&[Feature::Kanban]);
    let mut o = FeatureOffers {
        mcp_tickets: times(MCP_TICKETS, H),
        ..Default::default()
    };
    let got = offer_of(&o, &kanban, &Live::default(), NOW).unwrap();
    assert_eq!(
        (got.feature, got.signal, got.count),
        (Feature::Kanban, Signal::McpTickets, 2)
    );
    // Just under: one.
    o.mcp_tickets.pop();
    assert_eq!(offer_of(&o, &kanban, &Live::default(), NOW), None);
    // Two, but one older than the week.
    o.mcp_tickets.push(NOW - MCP_TICKETS_WITHIN - 1);
    assert_eq!(offer_of(&o, &kanban, &Live::default(), NOW), None);
    // The Kanban on: nothing to propose.
    o.mcp_tickets = times(5, H);
    assert_eq!(
        offer_of(&o, &Features::default(), &Live::default(), NOW),
        None
    );
}

#[test]
fn kanban_off_three_agents_started_on_long_messages_in_a_week_propose_it_two_do_not() {
    let kanban = off(&[Feature::Kanban]);
    let mut o = FeatureOffers {
        long_prompts: times(LONG_PROMPTS, DAY_MS),
        ..Default::default()
    };
    let got = offer_of(&o, &kanban, &Live::default(), NOW).unwrap();
    assert_eq!((got.signal, got.count), (Signal::LongPrompts, 3));
    o.long_prompts.pop();
    assert_eq!(offer_of(&o, &kanban, &Live::default(), NOW), None);
}

#[test]
fn docs_off_a_spec_changed_on_three_days_proposes_it_two_days_do_not() {
    let docs = off(&[Feature::Docs]);
    // Read from the commits: three of one day count once.
    let commits = [NOW, NOW - H, NOW - 2 * H, NOW - DAY_MS, NOW - 2 * DAY_MS];
    assert_eq!(days_within(&commits, NOW, SPEC_WITHIN, 0), 3);
    let live = |days: u32| Live {
        spec_days: Some((days, "demo".into())),
        ..Default::default()
    };
    let got = offer_of(&FeatureOffers::default(), &docs, &live(3), NOW).unwrap();
    assert_eq!(
        (got.feature, got.signal, got.count, got.project.as_deref()),
        (Feature::Docs, Signal::SpecEdits, 3, Some("demo"))
    );
    assert_eq!(
        offer_of(&FeatureOffers::default(), &docs, &live(2), NOW),
        None
    );
    // Older than two weeks: not counted.
    assert_eq!(
        days_within(&[NOW - SPEC_WITHIN - 1], NOW, SPEC_WITHIN, 0),
        0
    );
}

#[test]
fn docs_off_three_documents_dropped_in_two_weeks_propose_it_two_do_not() {
    let docs = off(&[Feature::Docs]);
    let mut o = FeatureOffers {
        documents: times(DOCUMENTS, 4 * DAY_MS),
        ..Default::default()
    };
    let got = offer_of(&o, &docs, &Live::default(), NOW).unwrap();
    assert_eq!((got.signal, got.count), (Signal::Documents, 3));
    o.documents.pop();
    assert_eq!(offer_of(&o, &docs, &Live::default(), NOW), None);
    // Documents, not images.
    for d in [
        "cahier.PDF",
        "notes.md",
        "plan.docx",
        "budget.xlsx",
        "liste.csv",
    ] {
        assert!(is_document(d), "{d}");
    }
    for d in ["capture.png", "photo.jpg", "main.rs", "sans-extension"] {
        assert!(!is_document(d), "{d}");
    }
}

#[test]
fn cost_off_one_usage_limit_in_a_week_proposes_it_none_does_not() {
    let cost = off(&[Feature::Cost]);
    let mut o = FeatureOffers {
        limits: vec![NOW - LIMITS_WITHIN],
        ..Default::default()
    };
    let got = offer_of(&o, &cost, &Live::default(), NOW).unwrap();
    assert_eq!(
        (got.feature, got.signal, got.count),
        (Feature::Cost, Signal::UsageLimit, 1)
    );
    // Just under: none in the week (the one met a millisecond too early).
    o.limits = vec![NOW - LIMITS_WITHIN - 1];
    assert_eq!(offer_of(&o, &cost, &Live::default(), NOW), None);
}

#[test]
fn workflows_off_a_project_with_two_skills_commands_or_agents_proposes_it_one_does_not() {
    let dir = test_dir("offers-agent-files");
    let claude = dir.join(".claude");
    std::fs::create_dir_all(claude.join("skills").join("relire")).unwrap();
    std::fs::create_dir_all(claude.join("commands")).unwrap();
    // What does not count: its settings, a skill without its SKILL.md, a command that is not Markdown.
    std::fs::write(claude.join("settings.json"), "{}").unwrap();
    std::fs::create_dir_all(claude.join("skills").join("vide")).unwrap();
    std::fs::write(claude.join("commands").join("notes.txt"), "").unwrap();
    std::fs::write(
        claude.join("skills").join("relire").join("SKILL.md"),
        "# Relire",
    )
    .unwrap();
    assert_eq!(agent_files(&dir), 1);
    let workflows = off(&[Feature::Workflows]);
    let live = |n: u32| Live {
        agent_files: Some((n, "demo".into())),
        ..Default::default()
    };
    assert_eq!(
        offer_of(
            &FeatureOffers::default(),
            &workflows,
            &live(agent_files(&dir)),
            NOW
        ),
        None
    );
    // An agent of `.agents/`: two.
    std::fs::create_dir_all(dir.join(".agents").join("agents")).unwrap();
    std::fs::write(dir.join(".agents").join("agents").join("relecteur.md"), "").unwrap();
    assert_eq!(agent_files(&dir), 2);
    let got = offer_of(
        &FeatureOffers::default(),
        &workflows,
        &live(agent_files(&dir)),
        NOW,
    )
    .unwrap();
    assert_eq!(
        (got.feature, got.signal, got.count),
        (Feature::Workflows, Signal::AgentFiles, 2)
    );
}

#[test]
fn scheduled_off_the_same_message_at_the_same_hour_three_days_in_a_row_proposes_it() {
    let scheduled = off(&[Feature::Scheduled]);
    let hash = fingerprint("Fais la revue des PR ouvertes du projet").unwrap();
    // The words alone: case and spaces do not change it; a word does; a message too short has none.
    assert_eq!(
        fingerprint("  fais LA revue des PR\nouvertes du projet ").as_deref(),
        Some(hash.as_str())
    );
    assert_ne!(
        fingerprint("Fais la revue des PR fermées du projet"),
        Some(hash.clone())
    );
    assert_eq!(fingerprint("continue"), None);
    let at = |days_ago: i64, minutes: i64| MessageSeen {
        hash: hash.clone(),
        at: NOW - days_ago * DAY_MS + minutes * MINUTE_MS,
    };
    // Three days in a row, within the hour (from 14 h 30 to 15 h 30).
    let mut o = FeatureOffers {
        messages: vec![at(2, -30), at(1, 30), at(0, 0)],
        ..Default::default()
    };
    let got = offer_of(&o, &scheduled, &Live::default(), NOW).unwrap();
    assert_eq!(
        (got.feature, got.signal, got.count),
        (Feature::Scheduled, Signal::SameMessage, 3)
    );
    // Just under: two days.
    let two = FeatureOffers {
        messages: vec![at(1, 30), at(0, 0)],
        ..Default::default()
    };
    assert_eq!(offer_of(&two, &scheduled, &Live::default(), NOW), None);
    // Three days, but one missed between.
    let gap = FeatureOffers {
        messages: vec![at(3, 0), at(1, 0), at(0, 0)],
        ..Default::default()
    };
    assert_eq!(offer_of(&gap, &scheduled, &Live::default(), NOW), None);
    // Three days in a row, a minute over the hour.
    let late = FeatureOffers {
        messages: vec![at(2, -30), at(1, 31), at(0, 0)],
        ..Default::default()
    };
    assert_eq!(offer_of(&late, &scheduled, &Live::default(), NOW), None);
    // Three different messages: none.
    o.messages[0].hash = fingerprint("Autre chose à faire aujourd'hui").unwrap();
    assert_eq!(offer_of(&o, &scheduled, &Live::default(), NOW), None);
    // The Kanban off with it: the Kanban comes first (the runs are tickets); never proposed alone.
    o.messages[0].hash = hash.clone();
    assert_eq!(
        offer_of(
            &o,
            &off(&[Feature::Kanban, Feature::Scheduled]),
            &Live::default(),
            NOW
        ),
        None
    );
}

#[test]
fn overseer_off_agents_waiting_on_three_projects_at_once_propose_it_two_do_not() {
    let overseer = off(&[Feature::Overseer]);
    let seen = |projects: u32| FeatureOffers {
        waiting: vec![WaitingSeen {
            at: NOW - DAY_MS,
            projects,
        }],
        ..Default::default()
    };
    let got = offer_of(&seen(WAITING_PROJECTS), &overseer, &Live::default(), NOW).unwrap();
    assert_eq!(
        (got.feature, got.signal, got.count),
        (Feature::Overseer, Signal::WaitingProjects, 3)
    );
    assert_eq!(
        offer_of(
            &seen(WAITING_PROJECTS - 1),
            &overseer,
            &Live::default(),
            NOW
        ),
        None
    );
    // Seen more than a week ago: gone.
    let old = FeatureOffers {
        waiting: vec![WaitingSeen {
            at: NOW - WAITING_WITHIN - 1,
            projects: 4,
        }],
        ..Default::default()
    };
    assert_eq!(offer_of(&old, &overseer, &Live::default(), NOW), None);
}

/// Every signal of the register at its threshold, every feature off.
fn all_noticed() -> (FeatureOffers, Live) {
    let hash = fingerprint("Fais la revue des PR ouvertes du projet").unwrap();
    let o = FeatureOffers {
        mcp_tickets: times(2, H),
        documents: times(3, H),
        limits: times(1, H),
        waiting: vec![WaitingSeen {
            at: NOW,
            projects: 3,
        }],
        messages: (0..3)
            .map(|d| MessageSeen {
                hash: hash.clone(),
                at: NOW - d * DAY_MS,
            })
            .collect(),
        ..Default::default()
    };
    let live = Live {
        spec_days: Some((3, "demo".into())),
        agent_files: Some((2, "demo".into())),
    };
    (o, live)
}

#[test]
fn one_at_a_time_in_the_order_of_the_register_and_one_a_day() {
    let (mut o, live) = all_noticed();
    let mut features = Features::all(false);
    features.set(Feature::Kanban, false);
    let mut order = Vec::new();
    let mut now = NOW;
    // Each « Pas maintenant » lets the next one come, the day after.
    while let Some(got) = candidate(&o, &features, &none, &live, Moment::default(), now, 0) {
        order.push(got.feature);
        apply(
            &mut o,
            OfferAct::Shown {
                feature: got.feature,
            },
            now,
        );
        // One a day: none in the 24 hours after it was shown, whatever else holds.
        assert_eq!(
            candidate(
                &o,
                &features,
                &none,
                &live,
                Moment::default(),
                now + ONE_A_DAY_MS - 1,
                0
            ),
            None
        );
        apply(
            &mut o,
            OfferAct::Later {
                feature: got.feature,
            },
            now,
        );
        now += ONE_A_DAY_MS;
        if order.len() > 10 {
            break;
        }
    }
    // Tâches programmées waits for the Kanban; Éditeur, Statistiques and Voix have no signal.
    assert_eq!(
        order,
        [
            Feature::Kanban,
            Feature::Docs,
            Feature::Cost,
            Feature::Workflows,
            Feature::Overseer
        ]
    );
}

#[test]
fn never_during_a_turn_on_screen_nor_while_an_agent_waits_for_a_permission() {
    let (o, live) = all_noticed();
    let kanban = off(&[Feature::Kanban]);
    for moment in [
        Moment {
            turn: true,
            permission: false,
        },
        Moment {
            turn: false,
            permission: true,
        },
    ] {
        assert_eq!(
            candidate(&o, &kanban, &none, &live, moment, NOW, 0),
            None,
            "{moment:?}"
        );
    }
    assert!(candidate(&o, &kanban, &none, &live, Moment::default(), NOW, 0).is_some());
}

#[test]
fn not_now_waits_a_week_for_the_same_one_and_never_is_for_good_until_given_back() {
    let (mut o, live) = all_noticed();
    let kanban = off(&[Feature::Kanban]);
    // The tickets refused go on coming: the signal holds whenever it is asked.
    let at = |o: &mut FeatureOffers, now: i64| {
        o.mcp_tickets = vec![now, now - H];
        offer_of(o, &kanban, &live, now)
    };
    assert!(apply(
        &mut o,
        OfferAct::Later {
            feature: Feature::Kanban
        },
        NOW
    ));
    assert_eq!(at(&mut o, NOW + LATER_MS - 1), None);
    assert_eq!(at(&mut o, NOW + LATER_MS).unwrap().feature, Feature::Kanban);
    // « Ne plus proposer »: never, a year later neither.
    apply(
        &mut o,
        OfferAct::Never {
            feature: Feature::Kanban,
        },
        NOW,
    );
    assert_eq!(at(&mut o, NOW + 365 * DAY_MS), None);
    // Given back from « Propositions écartées »: it may come again.
    assert!(apply(
        &mut o,
        OfferAct::Restore {
            feature: Feature::Kanban
        },
        NOW
    ));
    assert!(at(&mut o, NOW + DAY_MS).is_some());
}

#[test]
fn a_feature_imposed_off_is_never_proposed() {
    let (o, live) = all_noticed();
    let kanban = off(&[Feature::Kanban]);
    let imposed = |f: Feature| f == Feature::Kanban;
    assert_eq!(
        candidate(&o, &kanban, &imposed, &live, Moment::default(), NOW, 0),
        None
    );
    // Tâches programmées, the Kanban imposed: never either.
    let scheduled = off(&[Feature::Scheduled]);
    assert_eq!(
        candidate(&o, &scheduled, &imposed, &live, Moment::default(), NOW, 0),
        None
    );
}

#[test]
fn what_is_noticed_is_kept_two_weeks() {
    let (mut o, _) = all_noticed();
    o.limits.push(NOW - KEPT_MS - 1);
    prune(&mut o, NOW);
    assert_eq!(o.limits, vec![NOW]);
    prune(&mut o, NOW + KEPT_MS + 1);
    assert!(o.mcp_tickets.is_empty() && o.messages.is_empty() && o.waiting.is_empty());
}

fn managed_kanban_off() -> Managed {
    Managed {
        values: [(
            "features.kanban".to_owned(),
            (json!(false), "the machine's policy".to_owned()),
        )]
        .into(),
        warnings: Vec::new(),
    }
}

fn switch(core: &Arc<Core>, f: Feature, on: bool) {
    let mut s = core.settings.read().clone();
    s.features.get_or_insert_with(Default::default).set(f, on);
    core.save_settings(s).unwrap();
}

#[tokio::test]
async fn the_core_notices_tickets_refused_and_proposes_the_kanban_which_activer_turns_on() {
    let h = harness("offers-kanban");
    switch(&h.core, Feature::Kanban, false);
    h.core.offer_note(Signal::McpTickets);
    assert_eq!(
        h.core.feature_offer(&[]).await.offer,
        None,
        "one is under the threshold"
    );
    h.core.offer_note(Signal::McpTickets);
    let got = h.core.feature_offer(&[]).await.offer.unwrap();
    assert_eq!(
        (got.feature, got.signal, got.count),
        (Feature::Kanban, Signal::McpTickets, 2)
    );
    // Shown: the day's one; « Activer » turns the Kanban on and gives the settings back.
    h.core
        .feature_offer_act(OfferAct::Shown {
            feature: Feature::Kanban,
        })
        .unwrap();
    let v = h
        .core
        .feature_offer_act(OfferAct::Enable {
            feature: Feature::Kanban,
        })
        .unwrap();
    assert!(h.core.feature_on(Feature::Kanban));
    assert!(v.settings.unwrap().feature(Feature::Kanban));
    // A feature on: nothing more is noticed of it.
    h.core.offer_note(Signal::McpTickets);
    assert_eq!(h.core.onboarding.read().offers.mcp_tickets.len(), 2);
}

#[tokio::test]
async fn a_long_first_message_by_hand_counts_but_not_a_second_one_nor_a_short_one() {
    let h = harness("offers-messages");
    let (p, _) = h.project(false).await;
    switch(&h.core, Feature::Kanban, false);
    switch(&h.core, Feature::Docs, false);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let long = "x".repeat(LONG_PROMPT_CHARS);
    // Just under: 299 characters.
    h.core.offer_note_message(&a, &long[1..], &[]);
    assert!(h.core.onboarding.read().offers.long_prompts.is_empty());
    h.core
        .offer_note_message(&a, &long, &["cahier.pdf".into(), "capture.png".into()]);
    {
        let o = h.core.onboarding.read().offers.clone();
        assert_eq!(o.long_prompts.len(), 1);
        assert_eq!(o.documents.len(), 1, "the image is not a document");
    }
    // Its second message is not a start.
    h.core.agent(&a).unwrap().lock().meta.prompts = 1;
    h.core.offer_note_message(&a, &long, &[]);
    assert_eq!(h.core.onboarding.read().offers.long_prompts.len(), 1);
    // Nothing of the text is kept: its fingerprint alone (Tâches programmées is off with the Kanban).
    let kept = serde_json::to_string(&h.core.onboarding.read().offers).unwrap();
    assert!(!kept.contains("xxxx"), "{kept}");
    assert_eq!(h.core.onboarding.read().offers.messages.len(), 3);
}

#[tokio::test]
async fn a_spec_changed_in_commits_on_three_days_is_read_from_the_project() {
    let h = harness("offers-spec");
    let (_, repo) = h.project(false).await;
    switch(&h.core, Feature::Docs, false);
    let commit = |days_ago: i64, name: &str| {
        let at = (now_ms() / 1000) - days_ago * 86_400;
        std::fs::create_dir_all(repo.join("docs")).unwrap();
        std::fs::write(repo.join("docs").join(name), name).unwrap();
        let date = format!("@{at} +0000");
        let out = crate::proc::output_in_files(
            std::process::Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(["add", "-A"]),
        )
        .unwrap();
        assert!(out.status.success());
        let out = crate::proc::output_in_files(
            std::process::Command::new("git")
                .arg("-C")
                .arg(&repo)
                .env("GIT_AUTHOR_DATE", &date)
                .env("GIT_COMMITTER_DATE", &date)
                .args([
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "user.name=t",
                    "commit",
                    "-qm",
                    name,
                ]),
        )
        .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    commit(3, "a.md");
    commit(2, "b.md");
    let (now, offset) = (now_ms(), local_offset_min());
    let live = h.core.offers_live(now, offset).await;
    assert_eq!(live.spec_days.as_ref().map(|(d, _)| *d), Some(2));
    assert_eq!(h.core.feature_offer(&[]).await.offer, None);
    commit(1, "c.md");
    // Read again after its hour.
    SPEC_READS.lock().clear();
    let got = h.core.feature_offer(&[]).await.offer.unwrap();
    assert_eq!(
        (got.signal, got.count, got.project.as_deref()),
        (Signal::SpecEdits, 3, Some("demo"))
    );
}

#[tokio::test]
async fn never_during_the_turn_of_an_agent_on_screen() {
    let h = harness("offers-turn");
    let (p, _) = h.project(false).await;
    switch(&h.core, Feature::Kanban, false);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.offer_note(Signal::McpTickets);
    h.core.offer_note(Signal::McpTickets);
    h.core.agent(&a).unwrap().lock().meta.status = AgentStatus::Running;
    assert_eq!(
        h.core.feature_offer(std::slice::from_ref(&a)).await.offer,
        None
    );
    // Working out of sight, it does not stand in the way.
    assert!(h.core.feature_offer(&[]).await.offer.is_some());
}

#[tokio::test]
async fn a_policy_that_imposes_the_kanban_off_forbids_its_proposal_and_its_activation() {
    let h = harness("offers-locked");
    h.core.set_managed(managed_kanban_off());
    assert!(!h.core.feature_on(Feature::Kanban));
    h.core.offer_note(Signal::McpTickets);
    h.core.offer_note(Signal::McpTickets);
    assert_eq!(h.core.feature_offer(&[]).await.offer, None);
    // A client that would still ask: refused, the feature stays off.
    assert!(h
        .core
        .feature_offer_act(OfferAct::Enable {
            feature: Feature::Kanban
        })
        .is_err());
    assert!(!h.core.feature_on(Feature::Kanban));
}

#[tokio::test]
async fn ne_plus_proposer_holds_after_a_restart_and_reproposer_gives_it_back() {
    let dir = test_dir("offers-restart");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    let load = || {
        let app = mock_app();
        let (core, _rx) = Core::load(app.handle().clone(), data.clone());
        std::mem::forget(app);
        core
    };
    let core = load();
    switch(&core, Feature::Kanban, false);
    core.offer_note(Signal::McpTickets);
    core.offer_note(Signal::McpTickets);
    assert!(core.feature_offer(&[]).await.offer.is_some());
    let v = core
        .feature_offer_act(OfferAct::Never {
            feature: Feature::Kanban,
        })
        .unwrap();
    assert_eq!(v.never, [Feature::Kanban]);
    core.save_state().unwrap();
    core.stop_for_good();
    drop(core);
    let again = load();
    assert_eq!(again.feature_offers().never, [Feature::Kanban]);
    assert_eq!(again.feature_offer(&[]).await.offer, None);
    again
        .feature_offer_act(OfferAct::Restore {
            feature: Feature::Kanban,
        })
        .unwrap();
    assert!(again.feature_offers().never.is_empty());
    assert!(again.feature_offer(&[]).await.offer.is_some());
    again.stop_for_good();
}

#[tokio::test]
async fn nothing_noticed_leaves_the_machine() {
    let h = harness("offers-private");
    let (p, _) = h.project(false).await;
    h.core.telemetry.set_consent(Some(true));
    switch(&h.core, Feature::Kanban, false);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let before = h.core.telemetry.pending().len();
    h.core.offer_note(Signal::McpTickets);
    h.core.offer_note(Signal::McpTickets);
    h.core
        .offer_note_message(&a, "Fais la revue des PR ouvertes du projet", &[]);
    let got = h.core.feature_offer(&[]).await.offer.unwrap();
    for act in [
        OfferAct::Shown {
            feature: got.feature,
        },
        OfferAct::Later {
            feature: got.feature,
        },
        OfferAct::Never {
            feature: got.feature,
        },
        OfferAct::Restore {
            feature: got.feature,
        },
    ] {
        h.core.feature_offer_act(act).unwrap();
    }
    // No event of the usage statistics: what is noticed stays in `state.json`, by fingerprints and moments.
    let events: Vec<String> = h.core.telemetry.pending()[before..]
        .iter()
        .map(|e| e.to_string())
        .collect();
    assert!(
        events
            .iter()
            .all(|e| !e.contains("offer") && !e.contains("feature")),
        "{events:?}"
    );
    let saved = serde_json::to_string(&h.core.onboarding.read().offers).unwrap();
    assert!(!saved.contains("revue"), "{saved}");
}
