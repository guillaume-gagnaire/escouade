//! The costs of a team in the core (TS-16), against the real `escouade-team-server`: two data folders, Alice the
//! administrator and Bob a member, each its own core and its own project of the team. Only the totals of the team's
//! projects go, and the server keeps nothing of them it could read; an administrator reads every device's, a member
//! only its own; « par membre » is told before a name goes; a budget is weighed before a start, stops the autopilot
//! beyond it and warns the administrator in its box; offline, the totals wait, and go once.

use std::collections::{BTreeMap, BTreeSet};

use escouade_team::settings::Effective;
use serde_json::json;

use crate::agent::TurnRow;
use crate::teamcosts::{totals_of, CostRules, Names, BUDGETS, GRAIN, STOP};
use crate::teamdocs_tests::{holds, on_server, pair, Side};

const MODEL: &str = "claude-opus-zebra";

fn turn(cost: f64) -> TurnRow {
    TurnRow {
        engine: crate::model::EngineId::default(),
        model: MODEL.into(),
        input: 1000,
        cache: 500,
        output: 200,
        cost,
    }
}

fn rules(e: serde_json::Value) -> Effective {
    let mut out = Effective::default();
    for (k, v) in e.as_object().unwrap() {
        out.enforced.insert(k.clone(), v.clone());
    }
    out
}

/// The team's rules on both sides, as the version in force would give them.
fn set_rules(a: &Side, b: &Side, team: &str, e: serde_json::Value) {
    let e = rules(e);
    a.h.core.set_team_cost_rules(team, "ACME", Some(&e));
    b.h.core.set_team_cost_rules(team, "ACME", Some(&e));
}

/// A round: each reads the team, then sends and weighs (as the hourly pass does, forced).
async fn round(a: &Side, b: &Side, team: &str) {
    for _ in 0..2 {
        a.teams.sync(team).await.unwrap();
        b.teams.sync(team).await.unwrap();
        b.h.core.team_costs_pass(true).await;
        a.teams.sync(team).await.unwrap();
        a.h.core.team_costs_pass(true).await;
    }
}

#[test]
fn the_totals_are_sums_of_the_teams_projects_alone() {
    let now = crate::model::now_ms();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let line = |project: &str, agent: &str, cost: f64| crate::stats::TurnLine {
        ts: now,
        project: project.into(),
        agent: agent.into(),
        engine: "claude".into(),
        model: MODEL.into(),
        account: "principal".into(),
        input: 10,
        cache: 5,
        output: 2,
        cost,
    };
    let names = Names {
        boards: [("p1".to_string(), "b1".to_string())].into(),
        tickets: [("a1".to_string(), "API-7".to_string())].into(),
        plans: ["principal".to_string()].into(),
    };
    let quota = BTreeMap::from([(format!("{today}|claude"), 64.0)]);
    let lines = vec![
        line("p1", "a1", 1.0),
        line("p1", "a2", 2.0),
        line("perso", "a3", 9.0),
    ];
    let t = totals_of(
        &lines,
        &BTreeSet::from([today.clone()]),
        &names,
        &quota,
        None,
        7,
    );
    assert_eq!(t.member, None);
    assert_eq!(t.days.len(), 1);
    let d = &t.days[0];
    assert_eq!(d.rows.len(), 1, "one row: project, harness, model, plan");
    let r = &d.rows[0];
    assert_eq!((r.input, r.cache, r.output, r.cost), (20, 10, 4, 3.0));
    assert!(r.plan, "an account with a quota: an equivalent in API");
    assert_eq!(r.quota, Some(64.0));
    assert_eq!(d.tickets.len(), 1);
    assert_eq!(
        (d.tickets[0].ticket.as_str(), d.tickets[0].cost),
        ("API-7", 1.0)
    );
    // A personal project never goes: its 9 $ are nowhere.
    assert!(!serde_json::to_string(&t).unwrap().contains("perso"));
    // A day not asked for is not there.
    let none = totals_of(
        &lines,
        &BTreeSet::from(["2000-01-01".to_string()]),
        &names,
        &quota,
        None,
        7,
    );
    assert!(none.days[0].rows.is_empty());
}

#[test]
fn the_rules_read_the_team_file() {
    let r = CostRules::of(&rules(json!({
        (GRAIN): "member",
        (BUDGETS): { "team": 500, "projects": { "API": 200, "bad": -3 }, "members": { "*": 50 } },
        (STOP): true,
    })));
    assert!(r.by_member);
    assert_eq!(r.team, Some(500.0));
    assert_eq!(r.projects, BTreeMap::from([("api".to_string(), 200.0)]));
    assert_eq!(r.members["*"], 50.0);
    assert!(r.stop);
    // « Par projet » d'office.
    assert_eq!(CostRules::of(&Effective::default()), CostRules::default());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_the_totals_go_and_the_administrator_reads_them_a_member_its_own() {
    let (server, mut alice, mut bob, team) = pair("costs-go").await;
    on_server(&mut alice, &mut bob).await;
    bob.h
        .core
        .stats
        .record_turns("principal", "bob-agent", &bob.p.id, &[turn(0.42)]);
    // A turn of a personal project: it never goes.
    bob.h
        .core
        .stats
        .record_turns("principal", "bob-perso", "perso-project", &[turn(9.0)]);
    alice
        .h
        .core
        .stats
        .record_turns("principal", "alice-agent", &alice.p.id, &[turn(1.0)]);
    round(&alice, &bob, &team).await;

    let a = alice.h.core.team_costs_view(&team).await.unwrap();
    assert!(a.admin && a.holds);
    let total: f64 = a.records.iter().map(|r| r.cost).sum();
    assert!((total - 1.42).abs() < 1e-9, "{total}");
    assert!(
        a.records.iter().all(|r| r.member.is_none()),
        "« par projet »"
    );
    assert!(a.records.iter().all(|r| r.model == MODEL));
    let journal = alice.p.team_project.clone().unwrap();
    assert!(a.records.iter().all(|r| r.project == journal));

    // Bob, a member, sees himself alone, and what went of him, exactly.
    let b = bob.h.core.team_costs_view(&team).await.unwrap();
    assert!(!b.admin && !b.holds);
    assert!(b.records.iter().all(|r| r.mine));
    let mine: f64 = b.records.iter().map(|r| r.cost).sum();
    assert!((mine - 0.42).abs() < 1e-9, "{mine}");
    let sent = b.sent.expect("what went of Bob");
    let text = serde_json::to_string(&sent).unwrap();
    assert!(!text.contains("perso"));
    assert!(!text.contains(&bob.p.name));
    assert!(b.pending.is_empty());
    // The summary of the month: the projects and the team, never a member.
    let s = b.summary.expect("the summary");
    assert!((s.team.cost - 1.42).abs() < 1e-9);

    // The server keeps nothing it could read of them.
    let kept = server.kept();
    for secret in [MODEL, journal.as_str(), "bob-agent"] {
        assert!(!holds(&kept, secret), "the server keeps {secret}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn par_membre_is_told_before_a_name_goes() {
    let (_server, mut alice, mut bob, team) = pair("costs-grain").await;
    on_server(&mut alice, &mut bob).await;
    bob.h
        .core
        .stats
        .record_turns("principal", "bob-agent", &bob.p.id, &[turn(0.5)]);
    set_rules(&alice, &bob, &team, json!({ (GRAIN): "member" }));
    // Bob has not seen the screen: his totals go without his name.
    assert_eq!(
        bob.h.core.team_costs_untold(),
        vec![(team.clone(), "ACME".to_string())]
    );
    round(&alice, &bob, &team).await;
    let a = alice.h.core.team_costs_view(&team).await.unwrap();
    assert!(a.records.iter().all(|r| r.member.is_none()));
    // He saw it: his name goes.
    bob.h.core.team_costs_ack(&team);
    assert!(bob.h.core.team_costs_untold().is_empty());
    let b = bob.h.core.team_costs_view(&team).await.unwrap();
    assert!(b.told);
    round(&alice, &bob, &team).await;
    let a = alice.h.core.team_costs_view(&team).await.unwrap();
    let me = bob.teams.member_of(&team).unwrap();
    assert!(a
        .records
        .iter()
        .any(|r| r.member.as_deref() == Some(me.as_str())));
    // « Par projet » again: the next « par membre » is told again.
    set_rules(&alice, &bob, &team, json!({}));
    set_rules(&alice, &bob, &team, json!({ (GRAIN): "member" }));
    assert_eq!(bob.h.core.team_costs_untold().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_budget_is_weighed_before_a_start_stops_the_autopilot_and_warns_the_administrator() {
    let (_server, mut alice, mut bob, team) = pair("costs-budget").await;
    on_server(&mut alice, &mut bob).await;
    alice.h.core.settings.write().mcp_enabled = true;
    let name = bob.p.name.clone();
    let journal = bob.p.team_project.clone().unwrap();
    bob.h
        .core
        .stats
        .record_turns("principal", "bob-agent", &bob.p.id, &[turn(0.92)]);
    // A budget of 1 $ for the project: 92 %, told before a start, nothing stopped.
    set_rules(
        &alice,
        &bob,
        &team,
        json!({ (BUDGETS): { "projects": { (journal.clone()): 1 } }, (STOP): true }),
    );
    round(&alice, &bob, &team).await;
    let n = bob.h.core.team_cost_notice(&bob.p.id).expect("told");
    assert_eq!(
        n.text,
        format!("Le projet {name} a dépensé 92 % de son budget du mois")
    );
    assert!(!n.over && !n.stops);
    assert!(bob.h.core.team_cost_stop(&bob.p.id).is_none());
    // The administrator's box holds the warning, once.
    let warned = |s: &Side| {
        s.h.core
            .asks_all()
            .into_iter()
            .filter(|a| a.title.contains("92 %"))
            .count()
    };
    assert_eq!(warned(&alice), 1);
    assert_eq!(warned(&bob), 0, "a member's device warns no one");
    round(&alice, &bob, &team).await;
    assert_eq!(warned(&alice), 1, "once");

    // Beyond it: the autopilot of the project stops, a launch by hand is told first.
    bob.h
        .core
        .stats
        .record_turns("principal", "bob-agent", &bob.p.id, &[turn(0.2)]);
    round(&alice, &bob, &team).await;
    let n = bob.h.core.team_cost_notice(&bob.p.id).expect("told");
    assert!(n.over && n.stops);
    assert!(n.text.contains("112 %"), "{}", n.text);
    let why = bob.h.core.team_cost_stop(&bob.p.id).expect("stopped");
    assert!(why.contains("imposé par l’équipe ACME"), "{why}");
    assert!(alice
        .h
        .core
        .asks_all()
        .iter()
        .any(|a| a.title.contains("112 %")));

    // A budget of the team beyond it: every project of the team, by the lock of TS-11.
    set_rules(
        &alice,
        &bob,
        &team,
        json!({ (BUDGETS): { "team": 0.5 }, (STOP): true }),
    );
    bob.h.core.team_costs_pass(false).await;
    let locks = bob.h.core.locks_for(&bob.p.id);
    assert!(locks.rows["autoAssign"].contains(&STOP.to_string()));
    assert_eq!(
        locks.locks[STOP].value,
        Some(serde_json::Value::Bool(false))
    );
    assert_eq!(
        locks.locks[STOP].by,
        Some(crate::locks::LockedBy::Team {
            team: "ACME".into()
        })
    );
    // Under it again, the lock goes.
    set_rules(
        &alice,
        &bob,
        &team,
        json!({ (BUDGETS): { "team": 100 }, (STOP): true }),
    );
    bob.h.core.team_costs_pass(false).await;
    assert!(bob.h.core.team_cost_stop(&bob.p.id).is_none());
    assert!(!bob
        .h
        .core
        .locks_for(&bob.p.id)
        .rows
        .get("autoAssign")
        .is_some_and(|r| r.contains(&STOP.to_string())));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn offline_the_totals_wait_then_go_once() {
    let (mut server, mut alice, mut bob, team) = pair("costs-offline").await;
    on_server(&mut alice, &mut bob).await;
    round(&alice, &bob, &team).await;
    server.stop().await;
    bob.h
        .core
        .stats
        .record_turns("principal", "bob-agent", &bob.p.id, &[turn(0.3)]);
    bob.h.core.team_costs_pass(true).await;
    bob.h.core.team_costs_pass(true).await;
    let b = bob.h.core.team_costs_view(&team).await.unwrap();
    assert!(!b.pending.is_empty(), "they wait");
    server.again().await;
    round(&alice, &bob, &team).await;
    round(&alice, &bob, &team).await;
    let b = bob.h.core.team_costs_view(&team).await.unwrap();
    assert!(b.pending.is_empty());
    let a = alice.h.core.team_costs_view(&team).await.unwrap();
    let total: f64 = a.records.iter().map(|r| r.cost).sum();
    assert!((total - 0.3).abs() < 1e-9, "counted once: {total}");
}
