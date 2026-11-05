//! The CI of the targets and of the branches the tickets push (ESC-63): its reading in one word,
//! since when it is red, the autopilot's pause, the pushed branches followed, and, against a fake
//! GitHub served by its plugin, the pill of a target, the pause that holds the autopilot back, a
//! ticket « Valider et pousser » that follows its branch's CI, a job's log and « Relancer les jobs
//! en échec ».

use super::*;
use crate::agentprs_tests::github_project;
use crate::core_tests::{harness, wt_step, Harness};
use crate::i18n::Lang::{En, Fr};
use crate::integrations::fake::FakeServer;
use crate::tickets_tests::{draft, reviewed};
use serde_json::{json, Value};

fn check(name: &str, state: CheckState) -> Check {
    Check {
        name: name.into(),
        state,
        ..Default::default()
    }
}

#[test]
fn the_checks_of_a_branch_say_one_word_a_failure_first() {
    use CheckState::*;
    assert_eq!(overall(&[]), None);
    assert_eq!(overall(&[check("a", Skipped)]), None);
    assert_eq!(
        overall(&[check("a", Success), check("b", Skipped)]),
        Some(Success)
    );
    assert_eq!(
        overall(&[check("a", Success), check("b", Pending)]),
        Some(Pending)
    );
    assert_eq!(
        overall(&[check("a", Pending), check("b", Failure)]),
        Some(Failure)
    );
}

#[test]
fn red_since_the_first_failed_job_ended_and_kept_while_it_stays_red() {
    let mut ci = BranchCi::default();
    let mut failed = check("test", CheckState::Failure);
    failed.finished = Some(1_000);
    failed.job = Some("7".into());
    let third = Check {
        url: Some("https://ci.example.com/1".into()),
        ..check("jenkins", CheckState::Failure)
    };
    read_into(
        &mut ci,
        &[failed, third, check("lint", CheckState::Success)],
        5_000_000,
    );
    assert!(ci.red());
    assert_eq!(ci.red_since, Some(1_000_000));
    assert_eq!((ci.checks_total, ci.checks_failed), (3, 2));
    // A CI of another kind keeps no log: the card says it.
    assert_eq!(
        ci.failed
            .iter()
            .map(|f| (f.name.as_str(), f.log))
            .collect::<Vec<_>>(),
        [("test", true), ("jenkins", false)]
    );
    // Still red on the next commit: since the first time.
    read_into(&mut ci, &[check("test", CheckState::Failure)], 9_000_000);
    assert_eq!(ci.red_since, Some(1_000_000));
    // A reading that fails says why and keeps the state.
    read_failed(&mut ci, "GitHub injoignable".into(), 9_100_000);
    read_failed(&mut ci, "GitHub injoignable".into(), 9_200_000);
    assert!(ci.red());
    assert_eq!(ci.error_since, Some(9_100_000));
    // Green again: no longer red, the error gone.
    read_into(&mut ci, &[check("test", CheckState::Success)], 9_300_000);
    assert!(!ci.red());
    assert_eq!((ci.red_since, ci.error.as_deref()), (None, None));
    // Red without an end said: since this reading.
    read_into(&mut ci, &[check("test", CheckState::Failure)], 9_400_000);
    assert_eq!(ci.red_since, Some(9_400_000));
}

#[test]
fn the_pause_holds_only_when_asked_and_the_target_is_red() {
    let red = BranchCi {
        checks: Some(CheckState::Failure),
        ..Default::default()
    };
    let green = BranchCi {
        checks: Some(CheckState::Success),
        ..Default::default()
    };
    let mut s = BoardSettings::default();
    assert!(!s.pause_on_red_target, "off by default");
    assert!(!pause_holds(&s, Some(&red)));
    s.pause_on_red_target = true;
    assert!(pause_holds(&s, Some(&red)));
    assert!(!pause_holds(&s, Some(&green)));
    assert!(!pause_holds(&s, None));
    // Its jobs run again: it waits, no longer red.
    let mut again = red.clone();
    rerun_started(&mut again);
    assert_eq!(again.checks, Some(CheckState::Pending));
    assert!(!pause_holds(&s, Some(&again)));
}

fn pushed(id: &str, done_at: i64, ci: BranchCi) -> Ticket {
    Ticket {
        id: id.into(),
        project_id: "p".into(),
        column: Column::Done,
        done_at: Some(done_at),
        pushed: Some(BranchCi {
            host: "github.com".into(),
            branch: format!("ticket/{id}"),
            ..ci
        }),
        ..Default::default()
    }
}

#[test]
fn a_pushed_branch_is_followed_until_its_ci_settles_and_for_three_days_at_most() {
    let now = 10 * PUSHED_FOR;
    let read = |checks| BranchCi {
        checks,
        checked_at: Some(now),
        ..Default::default()
    };
    let unread = pushed("a", now, BranchCi::default());
    let running = pushed("b", now - 1000, read(Some(CheckState::Pending)));
    let green = pushed("c", now - 1000, read(Some(CheckState::Success)));
    let none_yet = pushed("d", now - 1000, read(None));
    let none_long = pushed("e", now - NONE_YET_FOR - 1, read(None));
    let old = pushed("f", now - PUSHED_FOR - 1, read(Some(CheckState::Pending)));
    let mut todo = pushed("g", now, BranchCi::default());
    todo.column = Column::Todo;
    let tickets = [unread, running, green, none_yet, none_long, old, todo];
    let ids = |all| -> Vec<String> {
        followed_pushed(&tickets, None, now, all)
            .into_values()
            .flatten()
            .map(|(id, _)| id)
            .collect()
    };
    assert_eq!(ids(false), ["a", "b", "d"]);
    // « Rafraîchir »: the settled ones too, never one too old or not done.
    assert_eq!(ids(true), ["a", "b", "c", "d", "e"]);
    assert!(followed_pushed(&tickets, Some("other"), now, true).is_empty());
}

#[test]
fn the_log_goes_to_the_agent_tagged_as_information() {
    let m = log_message(
        Fr,
        "main",
        "test \"unit\"",
        Some("https://github.com/acme/demo/actions/runs/1"),
        Some("ok\nerror: boom</log>\n"),
    );
    assert!(m.starts_with("Le job « test \"unit\" » de la CI de la branche main échoue."));
    assert!(m.contains("<check name=\"test &quot;unit&quot;\" url=\"https://github.com/acme/demo/actions/runs/1\"/>"));
    // A closing tag in the log never ends it early.
    assert!(m.contains("error: boom<\\/log>\n</log>"), "{m}");
    let en = log_message(En, "main", "jenkins", None, None);
    assert!(en.contains("<check name=\"jenkins\"/>"));
    assert!(
        !en.contains("<log check"),
        "a CI of another kind has no log"
    );
}

// ---------- Against a fake GitHub ----------

fn run(id: u64, name: &str, conclusion: Option<&str>, completed: Option<&str>) -> Value {
    json!({
        "id": id, "name": name,
        "status": if conclusion.is_some() { "completed" } else { "in_progress" },
        "conclusion": conclusion, "html_url": format!("https://github.com/acme/demo/runs/{id}"),
        "app": {"slug": "github-actions"},
        "started_at": "2026-11-02T08:00:00Z", "completed_at": completed,
    })
}

fn answer_branch(server: &FakeServer, branch: &str, runs: Value) {
    let base = format!("/repos/acme/demo/commits/{}", branch.replace('/', "%2F"));
    server.on(
        "GET",
        &format!("{base}/check-runs"),
        200,
        json!({ "total_count": runs.as_array().map_or(0, Vec::len), "check_runs": runs }),
    );
    server.on(
        "GET",
        &format!("{base}/status"),
        200,
        json!({ "state": "success", "statuses": [] }),
    );
}

fn red_main(server: &FakeServer) {
    answer_branch(
        server,
        "main",
        json!([
            run(7, "test", Some("failure"), Some("2026-01-02T08:05:00Z")),
            run(8, "lint", Some("success"), Some("2026-01-02T08:01:00Z")),
        ]),
    );
}

async fn target_ci(h: &Harness, project_id: &str) -> Option<BranchCi> {
    h.core.branch_cis_refresh(Some(project_id), true).await;
    h.core.target_ci_of(project_id)
}

/// The header's pill: red since its job ended, its log read, its failed jobs run again on a
/// click; and the autopilot paused while it is red, when the board asks it.
#[tokio::test]
async fn the_target_ci_shows_on_the_header_and_pauses_the_autopilot_when_asked() {
    let h = harness("bci-target");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    red_main(&server);
    let ci = target_ci(&h, &p.id).await.expect("a pill on the header");
    assert_eq!(ci.branch, "main");
    assert!(ci.red() && ci.rerun);
    assert_eq!(
        ci.red_since,
        Some(
            chrono::DateTime::parse_from_rfc3339("2026-01-02T08:05:00Z")
                .unwrap()
                .timestamp_millis()
        )
    );
    assert_eq!(h.core.target_cis().len(), 1);

    // The autopilot off first: on, with no pause yet, it may rightly start the ticket at once.
    h.set_board(&p.id, |s| s.autopilot = false);
    let one = h
        .core
        .ticket_create(&p.id, draft("Un [ok]", &[], 5))
        .await
        .unwrap();
    h.set_board(&p.id, |s| {
        s.autopilot = true;
        s.pause_on_red_target = true;
    });
    h.core.schedule_now().await;
    assert_eq!(h.ticket(&one.id).column, Column::Todo, "held back, red");

    // Its log, in reading only.
    server.on_text(
        "GET",
        "/repos/acme/demo/actions/jobs/7/logs",
        200,
        "step 1\nerror: boom\n",
        &[],
    );
    let of = CiOf {
        kind: "target".into(),
        id: p.id.clone(),
    };
    let log = h.core.ci_job_log(&of, "7").await;
    assert_eq!(log.data.as_deref(), Some("step 1\nerror: boom"));
    let detail = h.core.ci_detail(&of).await.data.unwrap();
    assert_eq!((detail.branch.as_str(), detail.checks.len()), ("main", 2));
    assert!(detail.can.rerun && detail.can.job_logs);

    // « Relancer les jobs en échec »: its run again, then the pill waits and the pause lifts.
    server.on(
        "GET",
        "/repos/acme/demo/actions/jobs/7",
        200,
        json!({ "id": 7, "run_id": 99 }),
    );
    server.on(
        "POST",
        "/repos/acme/demo/actions/runs/99/rerun-failed-jobs",
        201,
        json!({}),
    );
    assert_eq!(h.core.ci_rerun(&of).await.unwrap(), 1);
    assert!(server
        .writes()
        .iter()
        .any(|w| w.path() == "/repos/acme/demo/actions/runs/99/rerun-failed-jobs"));
    let ci = h.core.target_ci_of(&p.id).unwrap();
    assert_eq!((ci.checks, ci.red_since), (Some(CheckState::Pending), None));
    h.wait_ticket(&one.id, "started once the target is no longer red", |x| {
        x.column != Column::Todo
    })
    .await;
}

/// A pass under way starts nothing more once the pause is on, or the autopilot off: each start takes
/// seconds, and the board it decided on is no longer the one the user sees.
#[tokio::test]
async fn a_pass_under_way_starts_no_more_tickets_once_the_board_says_stop() {
    for (name, stop) in [
        (
            "bci-mid-pause",
            (|s| s.pause_on_red_target = true) as fn(&mut BoardSettings),
        ),
        ("bci-mid-off", |s| s.autopilot = false),
    ] {
        let h = harness(name);
        let server = FakeServer::start().await;
        let (p, _, _) = github_project(&h, &server, false).await;
        red_main(&server);
        assert!(target_ci(&h, &p.id).await.unwrap().red());
        // Every ticket's worktree setup waits for the test's go: the pass stays on its first start.
        h.set_worktree_steps(
            &p.id,
            vec![wt_step(
                r#"node -e "const fs = require('fs'); const go = require('path').join(process.env.ESCOUADE_PROJECT_DIR, '..', 'go'); setInterval(() => { if (fs.existsSync(go)) process.exit(0) }, 50); setTimeout(() => process.exit(1), 20000)""#,
                "",
            )],
            vec![],
        );
        h.set_board(&p.id, |s| s.autopilot = false);
        let a = h
            .core
            .ticket_create(&p.id, draft("Un [ok]", &[], 5))
            .await
            .unwrap();
        let b = h
            .core
            .ticket_create(&p.id, draft("Deux [ok]", &[], 5))
            .await
            .unwrap();
        // Both are to start in the same pass.
        h.set_board(&p.id, |s| s.autopilot = true);
        h.wait("the first one setting its worktree up", |h| {
            h.ticket(&a.id)
                .agent_id
                .is_some_and(|x| h.view(&x).setup.is_some())
        })
        .await;
        assert_eq!(h.ticket(&b.id).column, Column::Todo);
        h.set_board(&p.id, stop);
        std::fs::write(h.dir.join("go"), "").unwrap();
        h.wait_ticket(&a.id, "to test", |t| t.column == Column::Review)
            .await;
        h.wait_board_idle().await;
        assert_eq!(h.ticket(&b.id).column, Column::Todo, "{name}");
    }
}

/// A host out of reach: said on the pill, which keeps what it read last; a project without a
/// remote on a host Escouade reads has no pill.
#[tokio::test]
async fn a_hosts_error_is_said_on_the_pill_and_a_project_without_a_host_has_none() {
    let h = harness("bci-error");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    red_main(&server);
    assert!(target_ci(&h, &p.id).await.unwrap().red());
    server.on(
        "GET",
        "/repos/acme/demo/commits/main/check-runs",
        502,
        json!({ "message": "Bad gateway" }),
    );
    let ci = target_ci(&h, &p.id).await.unwrap();
    assert!(ci.red(), "the last state stays");
    assert!(
        ci.error.as_deref().unwrap().starts_with("GitHub"),
        "{:?}",
        ci.error
    );

    let alone = harness("bci-alone");
    let (other, _) = alone.project(false).await;
    assert_eq!(target_ci(&alone, &other.id).await, None);
}

/// « Valider et pousser »: the ticket is done, its branch's CI followed on its card until it
/// settles, its failed jobs run again on a click.
#[tokio::test]
async fn a_ticket_validated_and_pushed_follows_its_branchs_ci() {
    let h = harness("bci-pushed");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    h.set_board(&p.id, |s| s.action = "push".into());
    answer_branch(&server, "main", json!([]));
    answer_branch(
        &server,
        "ticket/dem-1",
        json!([run(11, "test", None, None)]),
    );
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Done, "{:?}", u.blocked);
    let ci = u.pushed.clone().expect("its branch followed");
    assert_eq!(
        (ci.branch.as_str(), ci.namespace.as_str(), ci.name.as_str()),
        ("ticket/dem-1", "acme", "demo")
    );
    h.core.branch_cis_refresh(Some(&p.id), false).await;
    let ci = h.ticket(&t.id).pushed.unwrap();
    assert_eq!(ci.checks, Some(CheckState::Pending));

    // It fails: red on the card, then settled (no longer read but on « Rafraîchir »).
    answer_branch(
        &server,
        "ticket/dem-1",
        json!([run(
            11,
            "test",
            Some("failure"),
            Some("2026-01-02T09:00:00Z")
        )]),
    );
    h.core.branch_cis_refresh(Some(&p.id), false).await;
    let ci = h.ticket(&t.id).pushed.unwrap();
    assert!(ci.red());
    assert_eq!(ci.failed.len(), 1);
    let before = server.requests().len();
    h.core.branch_cis_refresh(Some(&p.id), false).await;
    let read_again = server.requests()[before..]
        .iter()
        .any(|r| r.path().contains("ticket%2Fdem-1"));
    assert!(!read_again, "a settled branch is not read by itself");

    // « Relancer les jobs en échec » from its card.
    server.on(
        "GET",
        "/repos/acme/demo/actions/jobs/11",
        200,
        json!({ "id": 11, "run_id": 5 }),
    );
    server.on(
        "POST",
        "/repos/acme/demo/actions/runs/5/rerun-failed-jobs",
        201,
        json!({}),
    );
    let of = CiOf {
        kind: "pushed".into(),
        id: t.id.clone(),
    };
    assert_eq!(h.core.ci_rerun(&of).await.unwrap(), 1);
    assert_eq!(
        h.ticket(&t.id).pushed.unwrap().checks,
        Some(CheckState::Pending)
    );
}

/// A host whose plugin cannot run failed jobs again says so, in words the window shows.
#[tokio::test]
async fn a_rerun_the_host_cannot_do_is_said() {
    let h = harness("bci-unsupported");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    let of = CiOf {
        kind: "nothing".into(),
        id: p.id.clone(),
    };
    assert!(h.core.ci_rerun(&of).await.is_err());
    let of = CiOf {
        kind: "pushed".into(),
        id: "missing".into(),
    };
    assert!(h.core.ci_detail(&of).await.failure.is_some());
}

/// An ordinary agent's branch pushed without a pull request: its CI read with its pull requests,
/// the pill under its card; gone once it has a pull request.
#[tokio::test]
async fn an_agents_branch_pushed_without_a_pull_request_shows_its_ci() {
    let h = harness("bci-agent");
    let server = FakeServer::start().await;
    let (id, _, branch) = crate::agentprs_tests::github_agent(&h, &server).await;
    let no_pr = json!({ "data": { "repository": { "pullRequests": { "nodes": [] } } } });
    server.on("POST", "/graphql#PullsOfBranch", 200, no_pr);
    answer_branch(
        &server,
        &branch,
        json!([run(
            21,
            "test",
            Some("failure"),
            Some("2026-01-02T08:05:00Z")
        )]),
    );
    // Not pushed yet: nothing to show.
    h.core.agent_prs_refresh(&id, true).await.unwrap();
    assert_eq!(h.agent(&id).branch_ci, None);

    let dir = h.agent(&id).worktree.unwrap().path;
    crate::core_tests::git(
        std::path::Path::new(&dir),
        &["push", "-qu", "origin", &branch],
    );
    h.core.agent_prs_refresh(&id, true).await.unwrap();
    let ci = h.agent(&id).branch_ci.expect("its branch's CI");
    assert_eq!(ci.branch, branch);
    assert!(ci.red());
    let of = CiOf {
        kind: "agent".into(),
        id: id.clone(),
    };
    let detail = h.core.ci_detail(&of).await.data.unwrap();
    assert_eq!(detail.checks.len(), 1);

    // A pull request opened on it: its own CI shows, not the branch's.
    server.on(
        "POST",
        "/graphql#PullsOfBranch",
        200,
        json!({ "data": { "repository": { "pullRequests": { "nodes": [{
            "number": 7, "url": "https://github.com/acme/demo/pull/7", "title": "PR 7",
            "state": "OPEN", "isDraft": false, "headRefName": branch, "baseRefName": "main",
            "mergeable": "MERGEABLE", "isCrossRepository": false, "autoMergeRequest": null,
            "latestOpinionatedReviews": {"nodes": []}, "reviewThreads": {"nodes": []},
            "commits": {"nodes": [{"commit": {"statusCheckRollup": {"state": "FAILURE"}}}]},
        }] } } } }),
    );
    h.core.agent_prs_refresh(&id, true).await.unwrap();
    assert_eq!(h.agent(&id).prs.len(), 1);
    assert_eq!(h.agent(&id).branch_ci, None);
}

/// The window reads the event by its camelCase names, as it reads the CI itself.
#[test]
fn the_event_of_a_targets_ci_speaks_the_windows_names() {
    let e = UiEvent::TargetCi {
        project_id: "p1".into(),
        ci: Some(BranchCi {
            red_since: Some(5),
            ..Default::default()
        }),
    };
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["type"], "targetCi");
    assert_eq!(v["projectId"], "p1");
    assert_eq!(v["ci"]["redSince"], 5);
}
