//! The client of Gitea and Forgejo against a fake server fed with the answers of `tests/fixtures/forges/gitea/`
//! (written from the swagger documents of both, not recorded from an instance: see its README), once as a Gitea
//! 1.26 and once as a Forgejo 16, the two differing where the fixtures differ.
//!
//! The client is the plugin « Gitea / Forgejo »'s (`plugins/gitea`): an account's calls go to its
//! module by `ProcessForge`, as the core makes them, and the plugin calls the fake server.

use super::*;
use crate::integrations::fake::FakeServer;
use serde_json::json;

macro_rules! fx {
    ($name:literal) => {
        serde_json::from_str::<Value>(include_str!(concat!(
            "../../../tests/fixtures/forges/gitea/",
            $name,
            ".json"
        )))
        .unwrap()
    };
}

const JOB_LOG: &str = include_str!("../../../tests/fixtures/forges/gitea/job-log.txt");

const GITEA: &str = "1.26.0";
const FORGEJO: &str = "16.0.0-dev-798-568c49e1+gitea-1.22.0";

fn repo() -> RepoRef {
    RepoRef {
        namespace: "ada".into(),
        name: "notes".into(),
    }
}

/// The account of codeberg.org, its API being the fake server.
fn client(server: &FakeServer) -> Box<dyn Forge> {
    crate::plugins::builtin::served_forge(
        crate::plugins::builtin::GITEA,
        "gitea",
        &Credentials {
            url: "https://codeberg.org".into(),
            token: "tok_test".into(),
            api: Some(server.url.clone()),
            ..Default::default()
        },
    )
}

/// An instance of this version holding the pull request 7 of the fixtures: its reviews, its threads, its statuses and,
/// when its version has them, its Actions.
async fn instance(version: &str) -> FakeServer {
    let forgejo = version.contains("+gitea-");
    let server = FakeServer::start().await;
    server.on("GET", "/version", 200, json!({ "version": version }));
    server.on(
        "GET",
        "/repos/ada/notes/pulls/7",
        200,
        if forgejo {
            fx!("forgejo-pull")
        } else {
            fx!("pull")
        },
    );
    server.on(
        "GET",
        "/repos/ada/notes/pulls/7/reviews",
        200,
        fx!("reviews"),
    );
    server.on(
        "GET",
        "/repos/ada/notes/pulls/7/reviews/12/comments",
        200,
        fx!("review-12-comments"),
    );
    server.on(
        "GET",
        "/repos/ada/notes/issues/7/comments",
        200,
        fx!("issue-comments"),
    );
    let sha = "3f2a9c1d5e7b84a6f0c2d9e8b7a6543210fedcba";
    server.on(
        "GET",
        &format!("/repos/ada/notes/commits/{sha}/status"),
        200,
        fx!("combined-status"),
    );
    server.on(
        "GET",
        &format!("/repos/ada/notes/commits/{sha}/statuses"),
        200,
        fx!("statuses"),
    );
    server.on(
        "GET",
        "/repos/ada/notes/actions/runs",
        200,
        if forgejo {
            fx!("forgejo-runs")
        } else {
            fx!("gitea-runs")
        },
    );
    server.on(
        "GET",
        "/repos/ada/notes/actions/runs/900/jobs",
        200,
        if forgejo {
            fx!("forgejo-run-jobs")
        } else {
            fx!("gitea-run-jobs")
        },
    );
    server.on_text(
        "GET",
        "/repos/ada/notes/actions/jobs/5002/logs",
        200,
        JOB_LOG,
        &[],
    );
    server.on(
        "GET",
        "/repos/ada/notes/branches/main",
        200,
        fx!("branch-main"),
    );
    server.on(
        "GET",
        "/repos/ada/notes/branches/dev",
        200,
        fx!("branch-dev"),
    );
    server
}

fn writes_to(server: &FakeServer, path: &str) -> Vec<crate::integrations::fake::Request> {
    server
        .writes()
        .into_iter()
        .filter(|r| r.path() == path)
        .collect()
}

#[tokio::test]
async fn a_pull_request_is_read_with_its_reviews_threads_and_statuses_on_gitea_and_on_forgejo() {
    for version in [GITEA, FORGEJO] {
        let server = instance(version).await;
        let pr = client(&server).pull_request(&repo(), 7).await.unwrap();
        assert_eq!(pr.number, 7, "{version}");
        assert_eq!(pr.url, "https://codeberg.org/ada/notes/pulls/7");
        assert_eq!(pr.state, PrState::Open);
        assert!(!pr.draft && !pr.fork);
        assert_eq!(
            (pr.head.as_str(), pr.base.as_str()),
            ("feat/search", "main")
        );
        assert_eq!(pr.author, "ada");
        assert_eq!(pr.mergeable, Some(true));
        // grace approved then commented (a comment does not take an approval back); linus asks for changes;
        // mallory's review is still pending, so it counts for nothing.
        assert_eq!(pr.approvals, 1, "{version}");
        assert!(pr.changes_requested);
        assert_eq!(
            pr.open_threads, 1,
            "one of linus's two comments is resolved"
        );
        assert_eq!(pr.checks, Some(CheckState::Failure));
        assert_eq!(pr.created, 1_793_350_800);
        assert_eq!(
            server.requests()[0].headers["authorization"],
            "token tok_test"
        );
    }
}

#[tokio::test]
async fn a_merged_pull_request_is_merged_and_says_nothing_of_its_conflicts() {
    let server = instance(GITEA).await;
    server.on("GET", "/repos/ada/notes/pulls/7", 200, fx!("pull-merged"));
    let pr = client(&server).pull_request(&repo(), 7).await.unwrap();
    assert_eq!(pr.state, PrState::Merged);
    assert_eq!(pr.mergeable, None);
}

#[tokio::test]
async fn a_draft_and_a_fork_are_told_as_such() {
    let server = instance(GITEA).await;
    server.on("GET", "/repos/ada/notes/pulls/7", 200, fx!("pull-draft"));
    assert!(
        client(&server)
            .pull_request(&repo(), 7)
            .await
            .unwrap()
            .draft
    );
    server.on(
        "GET",
        "/repos/ada/notes/pulls/7",
        200,
        fx!("pull-from-fork"),
    );
    let pr = client(&server).pull_request(&repo(), 7).await.unwrap();
    assert!(pr.fork);
    assert_eq!(pr.head_repo, "bob/notes");
}

#[tokio::test]
async fn a_branchs_pull_requests_are_found_by_its_name_newest_first_and_the_open_ones_listed() {
    let server = instance(GITEA).await;
    server.on("GET", "/repos/ada/notes/pulls", 200, fx!("pulls-all"));
    let prs = client(&server)
        .pull_requests_of_branch(&repo(), "feat/search")
        .await
        .unwrap();
    assert_eq!(prs.iter().map(|p| p.number).collect::<Vec<_>>(), [7, 5]);
    assert!(prs[1].fork, "the older one came from bob's fork, same name");
    assert_eq!(prs[1].state, PrState::Closed);
    let asked = &server.requests()[0];
    assert_eq!(asked.query("state").as_deref(), Some("all"));

    // The other two have no review and no status.
    for n in [5, 6] {
        server.on(
            "GET",
            &format!("/repos/ada/notes/pulls/{n}/reviews"),
            200,
            json!([]),
        );
    }
    for sha in ["c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00", "d00dfeed"] {
        server.on(
            "GET",
            &format!("/repos/ada/notes/commits/{sha}/status"),
            200,
            fx!("combined-status-empty"),
        );
    }
    let open = client(&server).open_pull_requests(&repo()).await.unwrap();
    assert_eq!(open.len(), 3);
    assert_eq!(open[0].approvals, 1, "reviews and statuses come with each");
    assert_eq!(open[0].checks, Some(CheckState::Failure));
    assert_eq!(open[1].checks, None);
}

#[tokio::test]
async fn the_repository_a_fork_was_forked_from_is_read() {
    let server = instance(GITEA).await;
    server.on("GET", "/repos/bob/notes", 200, fx!("repo-fork"));
    let c = client(&server);
    let fork = RepoRef {
        namespace: "bob".into(),
        name: "notes".into(),
    };
    assert_eq!(c.parent(&fork).await.unwrap(), Some(repo()));
    server.on("GET", "/repos/ada/notes", 200, fx!("repo"));
    assert_eq!(c.parent(&repo()).await.unwrap(), None);
}

#[tokio::test]
async fn a_pull_request_opens_as_a_draft_with_its_reviewers_labels_and_issue() {
    for version in [GITEA, FORGEJO] {
        let server = instance(version).await;
        server.on("GET", "/repos/ada/notes/labels", 200, fx!("labels"));
        server.on("POST", "/repos/ada/notes/pulls", 201, fx!("pull-draft"));
        server.on(
            "POST",
            "/repos/ada/notes/pulls/7/requested_reviewers",
            201,
            json!([]),
        );
        let pr = client(&server)
            .create_pr(
                &repo(),
                &PullRequestDraft {
                    title: "Add search".into(),
                    body: "Adds the search page.".into(),
                    head: "feat/search".into(),
                    base: "main".into(),
                    draft: true,
                    reviewers: vec!["linus".into()],
                    labels: vec!["Agent".into(), "unknown".into()],
                    closes: Some(3),
                },
            )
            .await
            .unwrap();
        assert!(pr.draft && pr.number == 7, "{version}");
        let made = writes_to(&server, "/repos/ada/notes/pulls")[0].json();
        assert_eq!(made["title"], "WIP: Add search", "a draft is a WIP title");
        assert_eq!(made["body"], "Adds the search page.\n\nCloses #3");
        assert_eq!(made["head"], "feat/search");
        assert_eq!(made["base"], "main");
        assert_eq!(
            made["labels"],
            json!([1]),
            "labels by id; the unknown one left out"
        );
        assert!(
            made.get("draft").is_none(),
            "neither API has a draft field to write"
        );
        assert_eq!(
            writes_to(&server, "/repos/ada/notes/pulls/7/requested_reviewers")[0].json(),
            json!({ "reviewers": ["linus"] })
        );
    }
}

#[tokio::test]
async fn a_refused_reviewer_does_not_take_the_pull_request_back() {
    let server = instance(FORGEJO).await;
    server.on("POST", "/repos/ada/notes/pulls", 201, fx!("pull"));
    server.on(
        "POST",
        "/repos/ada/notes/pulls/7/requested_reviewers",
        422,
        json!({"message": "reviewer is the author"}),
    );
    let pr = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "Add search".into(),
                head: "feat/search".into(),
                base: "main".into(),
                reviewers: vec!["ada".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(pr.number, 7);
    let made = writes_to(&server, "/repos/ada/notes/pulls")[0].json();
    assert_eq!(made["title"], "Add search");
    assert_eq!(made["body"], "", "no « Closes » line without an issue");
}

#[tokio::test]
async fn a_pull_request_merges_by_each_strategy() {
    let server = instance(GITEA).await;
    server.on("POST", "/repos/ada/notes/pulls/7/merge", 200, Value::Null);
    let c = client(&server);
    for (strategy, name) in [
        (MergeStrategy::Merge, "merge"),
        (MergeStrategy::Squash, "squash"),
        (MergeStrategy::Rebase, "rebase"),
        (MergeStrategy::SemiLinear, "rebase-merge"),
    ] {
        c.merge(&repo(), 7, strategy).await.unwrap();
        let sent = writes_to(&server, "/repos/ada/notes/pulls/7/merge");
        assert_eq!(sent.last().unwrap().json(), json!({ "Do": name }));
    }
}

#[tokio::test]
async fn a_refused_merge_says_what_the_instance_said() {
    let server = instance(GITEA).await;
    server.on(
        "POST",
        "/repos/ada/notes/pulls/7/merge",
        405,
        json!({"message": "Not all required status checks successful"}),
    );
    let e = client(&server)
        .merge(&repo(), 7, MergeStrategy::Merge)
        .await
        .unwrap_err();
    assert!(
        format!("{e:#}").contains("Not all required status checks successful"),
        "{e:#}"
    );
}

#[tokio::test]
async fn auto_merge_is_scheduled_on_a_protected_branch_only() {
    for version in [GITEA, FORGEJO] {
        let server = instance(version).await;
        server.on("POST", "/repos/ada/notes/pulls/7/merge", 200, Value::Null);
        client(&server)
            .auto_merge(&repo(), 7, MergeStrategy::Squash)
            .await
            .unwrap();
        assert_eq!(
            writes_to(&server, "/repos/ada/notes/pulls/7/merge")[0].json(),
            json!({ "Do": "squash", "merge_when_checks_succeed": true }),
            "{version}"
        );

        // The base is not protected: nothing is asked of the instance, and the refusal says why.
        let mut open = fx!("pull");
        open["base"]["ref"] = json!("dev");
        server.on("GET", "/repos/ada/notes/pulls/7", 200, open);
        let before = server.writes().len();
        let e = client(&server)
            .auto_merge(&repo(), 7, MergeStrategy::Merge)
            .await
            .unwrap_err();
        assert!(
            e.downcast_ref::<crate::integrations::Refused>().is_some(),
            "a lasting refusal: {e:#}"
        );
        assert!(format!("{e:#}").contains("dev"), "{e:#}");
        assert_eq!(server.writes().len(), before);
    }
}

#[tokio::test]
async fn the_checks_come_with_their_actions_jobs_on_gitea_and_on_forgejo() {
    for version in [GITEA, FORGEJO] {
        let server = instance(version).await;
        let checks = client(&server).checks(&repo(), 7).await.unwrap();
        // The newest status of each context; the older pending « build » is not counted.
        assert_eq!(
            checks.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            [
                "CI / build (pull_request)",
                "CI / test (pull_request)",
                "lint/external"
            ],
            "{version}"
        );
        let state = |n: &str| checks.iter().find(|c| c.name == n).unwrap();
        assert_eq!(
            state("CI / build (pull_request)").state,
            CheckState::Success
        );
        assert_eq!(state("CI / test (pull_request)").state, CheckState::Failure);
        assert_eq!(state("lint/external").state, CheckState::Pending);
        // The status points at run 41, job 0 or 1: the jobs of that run, by their order.
        assert_eq!(
            state("CI / build (pull_request)").job.as_deref(),
            Some("5001")
        );
        assert_eq!(
            state("CI / test (pull_request)").job.as_deref(),
            Some("5002")
        );
        assert_eq!(
            state("lint/external").job,
            None,
            "a third-party CI keeps its log"
        );
        assert_eq!(
            state("lint/external").url.as_deref(),
            Some("https://ci.acme.dev/builds/9")
        );
        let asked = server
            .requests()
            .into_iter()
            .find(|r| r.path() == "/repos/ada/notes/actions/runs")
            .unwrap();
        assert_eq!(
            asked.query("head_sha").as_deref(),
            Some("3f2a9c1d5e7b84a6f0c2d9e8b7a6543210fedcba")
        );
    }
}

#[tokio::test]
async fn gitea_dates_its_jobs_and_forgejo_does_not() {
    let g = client(&instance(GITEA).await)
        .checks(&repo(), 7)
        .await
        .unwrap();
    let test = g.iter().find(|c| c.name.contains("test")).unwrap();
    assert!(test.started.is_some() && test.finished.is_some());
    let f = client(&instance(FORGEJO).await)
        .checks(&repo(), 7)
        .await
        .unwrap();
    let test = f.iter().find(|c| c.name.contains("test")).unwrap();
    assert!(test.started.is_none() && test.finished.is_none());
}

#[tokio::test]
async fn a_jobs_log_is_downloaded_where_the_instance_has_the_actions_api() {
    for version in [GITEA, FORGEJO] {
        let server = instance(version).await;
        let log = client(&server).job_log(&repo(), "5002").await.unwrap();
        assert_eq!(log, JOB_LOG, "{version}");
        assert!(log.contains("exit code 101"));
    }
}

#[tokio::test]
async fn the_actions_of_an_instance_too_old_are_absent_and_said_so() {
    // A Gitea 1.24 and a Forgejo 11 have statuses but no jobs, no logs: the checks come without a job, the log is
    // `Unsupported`, and the instance is not asked for them.
    for version in ["1.24.2", "11.0.3+gitea-1.22.0"] {
        let server = instance(version).await;
        let c = client(&server);
        let checks = c.checks(&repo(), 7).await.unwrap();
        assert_eq!(checks.len(), 3, "{version}");
        assert!(checks.iter().all(|c| c.job.is_none()), "{version}");
        let e = c.job_log(&repo(), "5002").await.unwrap_err();
        assert!(
            e.downcast_ref::<Unsupported>().is_some(),
            "{version}: {e:#}"
        );
        let e = c.rerun_failed(&repo(), 7).await.unwrap_err();
        assert!(
            e.downcast_ref::<Unsupported>().is_some(),
            "{version}: {e:#}"
        );
        assert!(
            server
                .requests()
                .iter()
                .all(|r| !r.path().contains("/actions/")),
            "{version}: no call to a route the version lacks"
        );
    }
}

#[tokio::test]
async fn the_failed_runs_are_run_again_on_gitea_only() {
    let server = instance(GITEA).await;
    server.on(
        "POST",
        "/repos/ada/notes/actions/runs/900/rerun-failed-jobs",
        200,
        json!({}),
    );
    assert_eq!(client(&server).rerun_failed(&repo(), 7).await.unwrap(), 1);
    assert_eq!(
        writes_to(
            &server,
            "/repos/ada/notes/actions/runs/900/rerun-failed-jobs"
        )
        .len(),
        1,
        "one run, one call, for its two jobs"
    );

    // Forgejo's API has no rerun; Gitea 1.25 has the logs but not the reruns.
    for version in [FORGEJO, "1.25.0"] {
        let server = instance(version).await;
        let e = client(&server).rerun_failed(&repo(), 7).await.unwrap_err();
        assert!(
            e.downcast_ref::<Unsupported>().is_some(),
            "{version}: {e:#}"
        );
        assert!(server.writes().is_empty());
    }
}

#[tokio::test]
async fn a_commit_without_statuses_has_no_checks() {
    let server = instance(GITEA).await;
    server.on(
        "GET",
        "/repos/ada/notes/commits/3f2a9c1d5e7b84a6f0c2d9e8b7a6543210fedcba/status",
        200,
        fx!("combined-status-empty"),
    );
    server.on(
        "GET",
        "/repos/ada/notes/commits/3f2a9c1d5e7b84a6f0c2d9e8b7a6543210fedcba/statuses",
        200,
        json!([]),
    );
    let c = client(&server);
    assert_eq!(c.pull_request(&repo(), 7).await.unwrap().checks, None);
    assert!(c.checks(&repo(), 7).await.unwrap().is_empty());
}

#[tokio::test]
async fn the_reviews_and_the_comments_of_a_pull_request_are_read() {
    let server = instance(FORGEJO).await;
    let c = client(&server);
    let reviews = c.reviews(&repo(), 7).await.unwrap();
    assert_eq!(
        reviews
            .iter()
            .map(|r| (r.author.as_str(), r.state))
            .collect::<Vec<_>>(),
        [
            ("grace", ReviewState::Approved),
            ("linus", ReviewState::ChangesRequested)
        ]
    );
    let comments = c.comments(&repo(), 7).await.unwrap();
    assert_eq!(comments.len(), 3);
    let line = comments.iter().find(|c| c.id == "201").unwrap();
    assert_eq!(line.path.as_deref(), Some("src/search.rs"));
    assert_eq!(line.line, Some(14));
    assert!(!line.resolved);
    assert!(comments.iter().find(|c| c.id == "202").unwrap().resolved);
    let talk = comments.iter().find(|c| c.id == "301").unwrap();
    assert_eq!((talk.path.clone(), talk.line), (None, None));
}

#[tokio::test]
async fn a_review_is_published_with_its_lines_and_a_comment_under_the_description() {
    let server = instance(GITEA).await;
    server.on(
        "POST",
        "/repos/ada/notes/pulls/7/reviews",
        200,
        json!({"id": 15}),
    );
    server.on(
        "POST",
        "/repos/ada/notes/issues/7/comments",
        201,
        json!({"id": 302}),
    );
    let c = client(&server);
    c.review(
        &repo(),
        7,
        &ReviewDraft {
            state: ReviewState::ChangesRequested,
            body: "Two things".into(),
            lines: vec![ReviewLine {
                path: "src/search.rs".into(),
                line: 14,
                body: "off by one".into(),
            }],
        },
    )
    .await
    .unwrap();
    assert_eq!(
        writes_to(&server, "/repos/ada/notes/pulls/7/reviews")[0].json(),
        json!({"event": "REQUEST_CHANGES", "body": "Two things",
               "comments": [{"path": "src/search.rs", "new_position": 14, "body": "off by one"}]})
    );
    c.comment(&repo(), 7, "Thanks").await.unwrap();
    assert_eq!(
        writes_to(&server, "/repos/ada/notes/issues/7/comments")[0].json(),
        json!({"body": "Thanks"})
    );
}

#[tokio::test]
async fn threads_are_answered_and_resolved_on_gitea_only() {
    let server = instance(GITEA).await;
    server.on(
        "POST",
        "/repos/ada/notes/pulls/7/comments/201/replies",
        201,
        json!({}),
    );
    server.on(
        "POST",
        "/repos/ada/notes/pulls/comments/201/resolve",
        204,
        Value::Null,
    );
    let c = client(&server);
    c.reply(&repo(), 7, "201", "Fixed").await.unwrap();
    c.resolve(&repo(), 7, "201").await.unwrap();
    assert_eq!(server.writes().len(), 2);

    for version in [FORGEJO, "1.25.0"] {
        let server = instance(version).await;
        let c = client(&server);
        for e in [
            c.reply(&repo(), 7, "201", "Fixed").await.unwrap_err(),
            c.resolve(&repo(), 7, "201").await.unwrap_err(),
        ] {
            assert!(
                e.downcast_ref::<Unsupported>().is_some(),
                "{version}: {e:#}"
            );
        }
        assert!(server.writes().is_empty());
    }
}

#[tokio::test]
async fn deployments_are_absent_on_both() {
    for version in [GITEA, FORGEJO] {
        let server = instance(version).await;
        let e = client(&server)
            .deployments(&repo(), "main")
            .await
            .unwrap_err();
        assert!(
            e.downcast_ref::<Unsupported>().is_some(),
            "{version}: {e:#}"
        );
        assert!(server.requests().is_empty(), "no call is made for them");
    }
    let caps = gitea::GiteaProvider.capabilities();
    assert!(!caps.deployments, "the code host declares them absent");
    assert!(caps.pull_requests && caps.auto_merge && caps.checks && caps.reviews);
    assert!(
        caps.job_logs && caps.rerun,
        "Actions are declared; an instance too old says it call by call"
    );
}

#[tokio::test]
async fn the_account_says_who_the_token_is_and_a_refused_one_is_a_401() {
    let server = instance(GITEA).await;
    server.on("GET", "/user", 200, fx!("user"));
    let me = client(&server).me().await.unwrap();
    assert_eq!((me.user.as_str(), me.label.as_str()), ("ada", "@ada"));
    assert_eq!(me.scopes, None, "neither says the rights of a token");

    server.on(
        "GET",
        "/user",
        401,
        json!({"message": "user does not exist [uid: 0]"}),
    );
    let e = client(&server).me().await.unwrap_err();
    assert!(refused(&e), "{e:#}");
}

#[tokio::test]
async fn the_instance_is_asked_its_version_once_per_client() {
    let server = instance(GITEA).await;
    let c = client(&server);
    c.job_log(&repo(), "5002").await.unwrap();
    c.job_log(&repo(), "5002").await.unwrap();
    c.checks(&repo(), 7).await.unwrap();
    let versions = server
        .requests()
        .iter()
        .filter(|r| r.path() == "/version")
        .count();
    assert_eq!(versions, 1);
}

/// A branch's checks (ESC-63): the statuses of its last commit with their Actions jobs; its failed
/// runs run again on Gitea.
#[tokio::test]
async fn a_branchs_checks_are_its_last_commits_statuses_and_run_again() {
    let server = instance(GITEA).await;
    let checks = client(&server).branch_checks(&repo(), "dev").await.unwrap();
    assert!(!checks.is_empty());
    assert!(checks.iter().any(|c| c.job.is_some()), "{checks:?}");
    server.on(
        "POST",
        "/repos/ada/notes/actions/runs/900/rerun-failed-jobs",
        200,
        json!({}),
    );
    let n = client(&server)
        .rerun_failed_of_branch(&repo(), "dev")
        .await
        .unwrap();
    assert!(n >= 1);
    assert_eq!(
        n,
        writes_to(
            &server,
            "/repos/ada/notes/actions/runs/900/rerun-failed-jobs"
        )
        .len()
    );
}

/// How a pull request may merge (ESC-62): the repository's merge styles, and auto-merge only when
/// its target branch is protected.
#[tokio::test]
async fn the_repositorys_styles_and_a_protected_branch_say_how_a_pull_request_may_merge() {
    let server = FakeServer::start().await;
    server.on("GET", "/version", 200, json!({ "version": "1.25.0" }));
    server.on(
        "GET",
        "/repos/ada/notes",
        200,
        json!({ "allow_merge_commits": true, "allow_rebase": false, "allow_rebase_explicit": false, "allow_squash_merge": true }),
    );
    server.on(
        "GET",
        "/repos/ada/notes/pulls/7",
        200,
        json!({ "number": 7, "state": "open", "base": { "ref": "main" }, "head": { "ref": "feat" } }),
    );
    server.on(
        "GET",
        "/repos/ada/notes/branches/main",
        200,
        json!({ "name": "main", "protected": false }),
    );
    let forge = client(&server);
    let o = forge.merge_options(&repo(), 7).await.unwrap();
    assert_eq!(
        o.strategies,
        vec![MergeStrategy::Merge, MergeStrategy::Squash]
    );
    assert!(!o.auto_merge, "not a protected branch");
    server.on(
        "GET",
        "/repos/ada/notes/branches/main",
        200,
        json!({ "name": "main", "protected": true }),
    );
    assert!(forge.merge_options(&repo(), 7).await.unwrap().auto_merge);
}
