use super::*;
use crate::forge::CheckState;
use crate::model::TicketPr;

fn dep(env: &str, state: &str, sha: &str) -> Deployment {
    Deployment {
        environment: env.into(),
        state: state.into(),
        sha: sha.into(),
        ..Default::default()
    }
}

#[test]
fn previews_and_review_apps_are_not_environments() {
    for n in [
        "Preview",
        "Preview – web",
        "review/fix-login",
        "deploy-preview-12",
        "pr-12",
    ] {
        assert!(is_preview(n), "{n}");
    }
    for n in ["production", "staging", "pr-office", "prod"] {
        assert!(!is_preview(n), "{n}");
    }
}

#[test]
fn production_is_told_by_its_name() {
    for n in ["production", "Production – web", "prod", "prod-eu"] {
        assert!(is_production(n), "{n}");
    }
    for n in ["staging", "products-demo", "preprod"] {
        assert!(!is_production(n), "{n}");
    }
}

#[test]
fn an_environment_runs_its_newest_success_and_says_a_newer_failure() {
    let mut staging = dep("staging", "success", "cccc");
    staging.url = Some("https://staging.example.com".into());
    let mut failure = dep("production", "failure", "dddd");
    failure.log = Some("https://example.com/job/2".into());
    let list = [
        dep("Preview", "success", "eeee"),
        failure,
        staging,
        dep("production", "success", "bbbb"),
        dep("production", "success", "aaaa"),
    ];
    let envs = summarize(&list);
    assert_eq!(
        envs.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["staging", "production"],
        "the preview left out, production last"
    );
    assert_eq!(envs[0].url.as_deref(), Some("https://staging.example.com"));
    assert!(!envs[0].failed);
    let prod = &envs[1];
    assert!(prod.production && prod.failed);
    assert_eq!(prod.running.as_ref().unwrap().sha, "bbbb");
    assert_eq!(
        prod.latest.as_ref().unwrap().log.as_deref(),
        Some("https://example.com/job/2")
    );
}

#[test]
fn a_deployment_under_way_is_no_failure() {
    let envs = summarize(&[
        dep("staging", "in_progress", "bbbb"),
        dep("staging", "success", "aaaa"),
    ]);
    assert!(!envs[0].failed);
    assert_eq!(envs[0].latest.as_ref().unwrap().state, "in_progress");
    assert_eq!(envs[0].running.as_ref().unwrap().sha, "aaaa");
}

#[test]
fn a_ticket_is_in_the_environments_whose_running_commit_holds_its_merge() {
    let envs = summarize(&[
        dep("staging", "success", "new"),
        dep("production", "success", "old"),
        dep("qa", "failure", "x"),
    ]);
    let placed = ticket_environments(&envs, |sha| match sha {
        "new" => Some(true),
        "old" => Some(false),
        _ => None,
    });
    assert_eq!(
        placed
            .iter()
            .map(|e| (e.name.as_str(), e.deployed))
            .collect::<Vec<_>>(),
        [("qa", false), ("staging", true), ("production", false)],
        "nothing runs in qa yet: not deployed there"
    );
}

#[test]
fn a_failure_is_news_once() {
    let ok = summarize(&[dep("production", "success", "a")]);
    let broken = summarize(&[
        dep("production", "failure", "b"),
        dep("production", "success", "a"),
    ]);
    assert_eq!(new_failures(&ok, &broken).len(), 1);
    assert!(new_failures(&broken, &broken).is_empty(), "already told");
    let again = summarize(&[
        dep("production", "failure", "c"),
        dep("production", "success", "a"),
    ]);
    assert_eq!(
        new_failures(&broken, &again).len(),
        1,
        "another deployment failed"
    );
    assert_eq!(
        failure_text(Lang::Fr, new_failures(&ok, &broken)[0]),
        "Déploiement en échec : production (b)"
    );
    assert_eq!(
        failure_text(Lang::En, new_failures(&ok, &broken)[0]),
        "Deployment failed: production (b)"
    );
}

#[test]
fn the_preview_is_a_deployment_of_the_branch_else_a_check_of_its_host() {
    let mut preview = dep("Preview", "success", "a");
    preview.url = Some("https://demo-git-fix.example.vercel.app".into());
    let mut prod = dep("production", "success", "a");
    prod.url = Some("https://example.com".into());
    assert_eq!(
        preview_of(&[prod.clone(), preview], &[]).as_deref(),
        Some("https://demo-git-fix.example.vercel.app")
    );
    let netlify = Check {
        name: "netlify/demo/deploy-preview".into(),
        state: CheckState::Success,
        url: Some("https://deploy-preview-12--demo.netlify.app".into()),
        ..Default::default()
    };
    let build = Check {
        name: "build".into(),
        state: CheckState::Success,
        url: Some("https://example.com/ci/1".into()),
        ..Default::default()
    };
    assert_eq!(
        preview_of(&[], &[build.clone(), netlify]).as_deref(),
        Some("https://deploy-preview-12--demo.netlify.app")
    );
    assert_eq!(
        preview_of(&[], &[build]),
        None,
        "a build's page is no preview"
    );
    let mut pending = dep("Preview", "pending", "a");
    pending.url = Some("https://x.example.com".into());
    assert_eq!(preview_of(&[pending], &[]), None, "not ready yet");
}

#[test]
fn a_done_ticket_is_followed_once_merged_and_for_thirty_days() {
    let now = 100 * 24 * 3600 * 1000;
    let t = Ticket {
        column: Column::Done,
        outcome_commit: Some("abc".into()),
        done_at: Some(now - 1000),
        ..Default::default()
    };
    assert!(followed(&t, now));
    assert!(!followed(
        &Ticket {
            outcome_commit: None,
            ..t.clone()
        },
        now
    ));
    assert!(!followed(
        &Ticket {
            done_at: Some(now - FOLLOWED_FOR - 1),
            ..t.clone()
        },
        now
    ));
    assert!(!followed(
        &Ticket {
            column: Column::Pr,
            ..t
        },
        now
    ));
}

#[test]
fn a_pull_request_merged_on_its_host_gives_its_ticket_the_commit_of_its_merge() {
    let mut t = Ticket {
        column: Column::Pr,
        pr: Some(TicketPr {
            number: 7,
            word: "PR".into(),
            target: "main".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let pr = crate::forge::PullRequest {
        number: 7,
        state: crate::forge::PrState::Merged,
        merge_commit: "0a1b2c3d".into(),
        ..Default::default()
    };
    crate::ticketprs::settle(&mut t, &pr, None, 1, Lang::Fr);
    assert_eq!(t.column, Column::Done);
    assert_eq!(t.outcome_commit.as_deref(), Some("0a1b2c3d"));
}

/// A project on GitHub (fake server), a ticket merged in main and pushed: placed in staging,
/// where main runs, and not yet in production; production's next deployment fails, notified once.
#[tokio::test]
async fn a_done_ticket_is_placed_in_its_environments_and_a_failure_is_notified_once() {
    use crate::core_tests::{git, harness};
    use serde_json::json;
    let h = harness("deploys");
    let server = crate::integrations::fake::FakeServer::start().await;
    let (p, r, _) = crate::agentprs_tests::github_project(&h, &server, false).await;
    let old = git(&r, &["rev-parse", "HEAD"]).trim().to_string();
    std::fs::write(r.join("feature.txt"), "x").unwrap();
    git(&r, &["add", "."]);
    git(&r, &["commit", "-q", "-m", "feat"]);
    let merge = git(&r, &["rev-parse", "HEAD"]).trim().to_string();
    git(&r, &["push", "-q", "origin", "main"]);
    h.core.tickets.write().push(Ticket {
        id: "t1".into(),
        project_id: p.id.clone(),
        key: "DEM-1".into(),
        column: Column::Done,
        outcome_commit: Some(merge.clone()),
        done_at: Some(now_ms()),
        ..Default::default()
    });
    let answer = |prod_state: &str| {
        let d = |id: u64, env: &str, sha: &str| {
            json!({"id": id, "environment": env, "sha": sha, "ref": "main",
                   "created_at": "2026-11-01T10:00:00Z", "creator": {"login": "ada"}})
        };
        server.on(
            "GET",
            "/repos/acme/demo/deployments",
            200,
            json!([
                d(3, "production", &merge),
                d(2, "staging", &merge),
                d(1, "production", &old)
            ]),
        );
        let s = |state: &str, url: &str| {
            json!([{"state": state, "environment_url": url,
                    "log_url": "https://github.com/acme/demo/actions/runs/1/job/2"}])
        };
        server.on(
            "GET",
            "/repos/acme/demo/deployments/3/statuses",
            200,
            s(prod_state, ""),
        );
        server.on(
            "GET",
            "/repos/acme/demo/deployments/2/statuses",
            200,
            s("success", "https://staging.example.com"),
        );
        server.on(
            "GET",
            "/repos/acme/demo/deployments/1/statuses",
            200,
            s("success", "https://example.com"),
        );
    };

    answer("in_progress");
    let envs = h.core.project_environments_read(&p.id).await;
    assert_eq!(envs.error, None);
    assert_eq!(envs.host, "GitHub");
    assert_eq!(
        envs.environments
            .iter()
            .map(|e| (e.name.as_str(), e.failed))
            .collect::<Vec<_>>(),
        [("staging", false), ("production", false)]
    );
    let t = h.core.ticket("t1").unwrap();
    assert_eq!(
        t.environments,
        [
            TicketEnv {
                name: "staging".into(),
                url: Some("https://staging.example.com".into()),
                deployed: true,
            },
            TicketEnv {
                name: "production".into(),
                url: Some("https://example.com".into()),
                deployed: false,
            },
        ]
    );
    assert!(h.alerts().is_empty());

    answer("failure");
    h.core.project_environments_read(&p.id).await;
    let alerts = h.alerts();
    assert_eq!(alerts.len(), 1, "{alerts:?}");
    assert!(alerts[0].contains("production"), "{alerts:?}");
    h.core.project_environments_read(&p.id).await;
    assert_eq!(h.alerts().len(), 1, "told once");

    answer("success");
    h.core.project_environments_read(&p.id).await;
    let t = h.core.ticket("t1").unwrap();
    assert!(
        t.environments.iter().all(|e| e.deployed),
        "{:?}",
        t.environments
    );
}

#[tokio::test]
async fn a_commit_still_missing_after_a_fetch_is_not_fetched_for_again() {
    use crate::core_tests::{git, harness};
    use serde_json::json;
    let h = harness("deploys-missing");
    let server = crate::integrations::fake::FakeServer::start().await;
    let (p, r, _) = crate::agentprs_tests::github_project(&h, &server, false).await;
    let merge = git(&r, &["rev-parse", "HEAD"]).trim().to_string();
    h.core.tickets.write().push(Ticket {
        id: "t1".into(),
        project_id: p.id.clone(),
        key: "DEM-1".into(),
        column: Column::Done,
        outcome_commit: Some(merge.clone()),
        done_at: Some(now_ms()),
        ..Default::default()
    });
    // A branch on the remote only: a fetch brings back its remote-tracking ref, which says it ran.
    git(&r, &["checkout", "-qb", "side"]);
    std::fs::write(r.join("side.txt"), "x").unwrap();
    git(&r, &["add", "."]);
    git(&r, &["commit", "-q", "-m", "side"]);
    git(&r, &["push", "-q", "origin", "side"]);
    git(&r, &["checkout", "-q", "main"]);
    git(&r, &["branch", "-qD", "side"]);
    let fetched = || {
        let out = std::process::Command::new("git")
            .args([
                "rev-parse",
                "--verify",
                "--quiet",
                "refs/remotes/origin/side",
            ])
            .current_dir(&r)
            .output()
            .unwrap();
        let yes = out.status.success();
        git(&r, &["update-ref", "-d", "refs/remotes/origin/side"]);
        yes
    };
    fetched();
    // Production runs a commit no remote has (its branch deleted since).
    let runs = |sha: &str| {
        server.on(
            "GET",
            "/repos/acme/demo/deployments",
            200,
            json!([{"id": 1, "environment": "production", "sha": sha, "ref": "main",
                    "created_at": "2026-11-01T10:00:00Z", "creator": {"login": "ada"}}]),
        );
        server.on(
            "GET",
            "/repos/acme/demo/deployments/1/statuses",
            200,
            json!([{"state": "success", "environment_url": "https://example.com"}]),
        );
    };
    runs(&"1".repeat(40));
    h.core.project_environments_read(&p.id).await;
    assert!(fetched(), "a commit not here is fetched for");
    h.core.project_environments_read(&p.id).await;
    assert!(!fetched(), "and not again once a fetch did not bring it");
    // Another commit not here: fetched for once more.
    runs(&"2".repeat(40));
    h.core.project_environments_read(&p.id).await;
    assert!(fetched(), "a new commit is fetched for");
}
