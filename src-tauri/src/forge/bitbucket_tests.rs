//! Bitbucket Cloud's client against a fake server fed with Bitbucket's answers (`tests/fixtures/forges/bitbucket/`,
//! written from its API documentation: nobody has an account to record from, see the README there), and with small
//! made-up ones for what they do not hold (a refusal, a limit, a merge that goes on in the background).
//!
//! The client is the plugin « Bitbucket »'s (`plugins/bitbucket`): an account's calls go to its
//! module by `ProcessForge`, as the core makes them, and the plugin calls the fake
//! server. What only the client itself shows (its pace, its count of the hour's calls) is read on it directly.

use super::*;
use crate::integrations::fake::FakeServer;
use serde_json::json;

fn fixture(name: &str) -> Value {
    let text = match name {
        "user" => include_str!("../../../tests/fixtures/forges/bitbucket/user.json"),
        "pullrequest" => include_str!("../../../tests/fixtures/forges/bitbucket/pullrequest.json"),
        "open" => include_str!("../../../tests/fixtures/forges/bitbucket/pullrequests-open.json"),
        "comments" => include_str!("../../../tests/fixtures/forges/bitbucket/comments.json"),
        "statuses" => include_str!("../../../tests/fixtures/forges/bitbucket/statuses.json"),
        "steps" => include_str!("../../../tests/fixtures/forges/bitbucket/steps.json"),
        "environments" => {
            include_str!("../../../tests/fixtures/forges/bitbucket/environments.json")
        }
        "deployments" => include_str!("../../../tests/fixtures/forges/bitbucket/deployments.json"),
        "branch" => include_str!("../../../tests/fixtures/forges/bitbucket/branch.json"),
        "error" => include_str!("../../../tests/fixtures/forges/bitbucket/error.json"),
        _ => panic!("no fixture {name}"),
    };
    serde_json::from_str(text).unwrap()
}

const BUILD_LOG: &str = include_str!("../../../tests/fixtures/forges/bitbucket/step-build-log.txt");
const TEST_LOG: &str = include_str!("../../../tests/fixtures/forges/bitbucket/step-test-log.txt");

const PR: &str = "/repositories/acme/demo/pullrequests";

fn repo() -> RepoRef {
    RepoRef {
        namespace: "acme".into(),
        name: "demo".into(),
    }
}

/// Bitbucket's account as the core serves it: by the plugin « Bitbucket », in its process, its API the fake server.
fn client(server: &FakeServer) -> Box<dyn Forge> {
    crate::plugins::builtin::served_forge(
        crate::plugins::builtin::BITBUCKET,
        "bitbucket",
        &Credentials {
            url: "https://bitbucket.org".into(),
            email: "ada@atlas.dev".into(),
            token: "ATATT_test".into(),
            api: Some(server.url.clone()),
        },
    )
}

fn route_pr(server: &FakeServer) {
    server.on("GET", &format!("{PR}/42"), 200, fixture("pullrequest"));
    server.on(
        "GET",
        &format!("{PR}/42/comments"),
        200,
        fixture("comments"),
    );
    server.on(
        "GET",
        &format!("{PR}/42/statuses"),
        200,
        fixture("statuses"),
    );
}

fn writes_to(server: &FakeServer, path: &str) -> Vec<crate::integrations::fake::Request> {
    server
        .writes()
        .into_iter()
        .filter(|r| r.path() == path)
        .collect()
}

#[test]
fn the_absence_of_auto_merge_is_declared() {
    let c = bitbucket::BitbucketProvider.capabilities();
    assert!(!c.auto_merge, "Bitbucket's API has no auto-merge");
    assert!(!c.rerun);
    assert!(c.pull_requests && c.draft && c.checks && c.job_logs);
    assert!(c.reviews && c.review_threads && c.deployments);
    assert!(c.merge_strategies.contains(&MergeStrategy::Squash));
    assert_eq!(
        bitbucket::BitbucketProvider.plugin(),
        Some(crate::plugins::builtin::BITBUCKET)
    );
}

#[tokio::test]
async fn auto_merge_and_rerun_are_not_something_the_client_does() {
    let server = FakeServer::start().await;
    let c = client(&server);
    let e = c
        .auto_merge(&repo(), 42, MergeStrategy::Squash)
        .await
        .unwrap_err();
    assert!(e.downcast_ref::<Unsupported>().is_some(), "{e:#}");
    let e = c.rerun_failed(&repo(), 42).await.unwrap_err();
    assert!(e.downcast_ref::<Unsupported>().is_some(), "{e:#}");
    assert!(
        server.requests().is_empty(),
        "nothing is asked of Bitbucket"
    );
}

#[tokio::test]
async fn the_account_says_who_it_is_with_its_email_and_token() {
    let server = FakeServer::start().await;
    server.on("GET", "/user", 200, fixture("user"));
    let id = client(&server).me().await.unwrap();
    assert_eq!(id.label, "@ada");
    assert_eq!(id.user, "557058:7f0c1c3a-1111-4b2b-9c0d-aaaaaaaaaaaa");
    assert_eq!(id.scopes, None);
    use base64::Engine;
    let basic = base64::engine::general_purpose::STANDARD.encode("ada@atlas.dev:ATATT_test");
    assert_eq!(
        server.requests()[0].headers["authorization"],
        format!("Basic {basic}")
    );
}

#[tokio::test]
async fn a_pull_request_is_read_with_its_reviews_threads_and_checks() {
    let server = FakeServer::start().await;
    route_pr(&server);
    let pr = client(&server).pull_request(&repo(), 42).await.unwrap();
    assert_eq!(pr.number, 42);
    assert_eq!(pr.state, PrState::Open);
    assert_eq!(pr.url, "https://bitbucket.org/acme/demo/pull-requests/42");
    assert_eq!(pr.title, "Add the login page");
    assert_eq!(
        (pr.head.as_str(), pr.base.as_str()),
        ("feature/login", "main")
    );
    assert!(!pr.draft);
    assert_eq!(pr.mergeable, None, "Bitbucket does not say before a merge");
    assert_eq!(pr.approvals, 1);
    assert!(pr.changes_requested);
    assert_eq!(
        pr.open_threads, 1,
        "one thread of a line is open: the resolved one, the conversation and the deleted one do not count"
    );
    assert!(!pr.auto_merge);
    assert_eq!(pr.checks, Some(CheckState::Failure));
    assert_eq!(pr.author, "ada");
    assert!(pr.created > 0);
    assert!(!pr.fork);
    assert_eq!(pr.head_repo, "");
}

#[tokio::test]
async fn a_merged_pull_request_costs_one_call() {
    let server = FakeServer::start().await;
    let mut merged = fixture("pullrequest");
    merged["state"] = json!("MERGED");
    server.on("GET", &format!("{PR}/42"), 200, merged);
    let pr = client(&server).pull_request(&repo(), 42).await.unwrap();
    assert_eq!(pr.state, PrState::Merged);
    assert_eq!(pr.checks, None);
    assert_eq!(server.requests().len(), 1);
    let mut declined = fixture("pullrequest");
    declined["state"] = json!("DECLINED");
    server.on("GET", &format!("{PR}/42"), 200, declined);
    let pr = client(&server).pull_request(&repo(), 42).await.unwrap();
    assert_eq!(pr.state, PrState::Closed);
}

#[tokio::test]
async fn a_pull_request_that_does_not_exist_is_an_error_and_a_batch_leaves_it_out() {
    let server = FakeServer::start().await;
    route_pr(&server);
    server.on(
        "GET",
        &format!("{PR}/7"),
        404,
        json!({"type": "error", "error": {"message": "No pull request"}}),
    );
    let c = client(&server);
    let e = c.pull_request(&repo(), 7).await.unwrap_err();
    assert!(format!("{e:#}").contains("No pull request"), "{e:#}");
    let both = c.pull_requests(&repo(), &[7, 42]).await.unwrap();
    assert_eq!(both.iter().map(|p| p.number).collect::<Vec<_>>(), [42]);
    assert!(c.pull_requests(&repo(), &[]).await.unwrap().is_empty());
}

#[tokio::test]
async fn the_open_pull_requests_come_newest_first_with_their_fork_and_draft() {
    let server = FakeServer::start().await;
    server.on("GET", PR, 200, fixture("open"));
    for n in [42, 43] {
        server.on(
            "GET",
            &format!("{PR}/{n}/comments"),
            200,
            json!({"values": []}),
        );
        server.on(
            "GET",
            &format!("{PR}/{n}/statuses"),
            200,
            json!({"values": [{"key": "k", "name": "CI", "state": "SUCCESSFUL"}]}),
        );
    }
    let prs = client(&server).open_pull_requests(&repo()).await.unwrap();
    assert_eq!(prs.iter().map(|p| p.number).collect::<Vec<_>>(), [43, 42]);
    assert!(prs[0].draft);
    assert!(prs[0].fork);
    assert_eq!(prs[0].head_repo, "ada/demo-fork");
    assert!(!prs[1].fork);
    assert_eq!(prs[1].approvals, 1);
    assert!(prs.iter().all(|p| p.checks == Some(CheckState::Success)));
    let list = &server.requests()[0];
    assert_eq!(list.query("state").as_deref(), Some("OPEN"));
    assert_eq!(list.query("sort").as_deref(), Some("-updated_on"));
}

#[tokio::test]
async fn a_branchs_pull_requests_are_found_by_its_name_whatever_their_state() {
    let server = FakeServer::start().await;
    server.on("GET", PR, 200, json!({"values": [fixture("pullrequest")]}));
    server.on(
        "GET",
        &format!("{PR}/42/comments"),
        200,
        fixture("comments"),
    );
    server.on(
        "GET",
        &format!("{PR}/42/statuses"),
        200,
        fixture("statuses"),
    );
    let found = client(&server)
        .pull_requests_of_branch(&repo(), "feature/login")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].head, "feature/login");
    let list = &server.requests()[0];
    assert_eq!(
        list.query("q").as_deref(),
        Some("source.branch.name=\"feature/login\"")
    );
    let states: Vec<String> = list
        .target
        .split('&')
        .filter(|kv| kv.starts_with("state="))
        .map(str::to_string)
        .collect();
    assert_eq!(
        states,
        [
            "state=OPEN",
            "state=MERGED",
            "state=DECLINED",
            "state=SUPERSEDED"
        ]
    );
    assert_eq!(list.query("sort").as_deref(), Some("-created_on"));
}

#[tokio::test]
async fn a_forks_parent_is_read_from_the_repository() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repositories/ada/demo-fork",
        200,
        json!({"full_name": "ada/demo-fork", "parent": {"full_name": "acme/demo"}}),
    );
    server.on(
        "GET",
        "/repositories/acme/demo",
        200,
        json!({"full_name": "acme/demo"}),
    );
    let c = client(&server);
    let fork = RepoRef {
        namespace: "ada".into(),
        name: "demo-fork".into(),
    };
    assert_eq!(c.parent(&fork).await.unwrap(), Some(repo()));
    assert_eq!(c.parent(&repo()).await.unwrap(), None);
}

#[tokio::test]
async fn a_pull_request_opens_as_a_draft_with_its_reviewers_and_the_issue_it_closes() {
    let server = FakeServer::start().await;
    let mut made = fixture("pullrequest");
    made["draft"] = json!(true);
    made["id"] = json!(44);
    server.on("POST", PR, 201, made);
    let pr = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "Add the login page".into(),
                body: "Adds /login.".into(),
                head: "feature/login".into(),
                base: "main".into(),
                draft: true,
                reviewers: vec!["557058:grace".into(), "{uuid-linus}".into()],
                labels: vec!["ignored".into()],
                closes: Some(7),
            },
        )
        .await
        .unwrap();
    assert_eq!(pr.number, 44);
    assert!(pr.draft);
    let sent = server.writes()[0].json();
    assert_eq!(sent["title"], "Add the login page");
    assert_eq!(sent["description"], "Adds /login.\n\nCloses #7");
    assert_eq!(sent["source"], json!({"branch": {"name": "feature/login"}}));
    assert_eq!(sent["destination"], json!({"branch": {"name": "main"}}));
    assert_eq!(sent["draft"], true);
    assert_eq!(
        sent["reviewers"],
        json!([{"account_id": "557058:grace"}, {"uuid": "{uuid-linus}"}])
    );
    assert!(sent.get("labels").is_none(), "Bitbucket has no labels");
}

#[tokio::test]
async fn a_pull_request_is_not_a_draft_unless_asked() {
    let server = FakeServer::start().await;
    server.on("POST", PR, 201, fixture("pullrequest"));
    client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "t".into(),
                head: "a".into(),
                base: "main".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let sent = server.writes()[0].json();
    assert!(sent.get("draft").is_none() && sent.get("reviewers").is_none());
}

#[tokio::test]
async fn a_refused_reviewer_does_not_take_the_pull_request_back() {
    let server = FakeServer::start().await;
    server.on("POST", PR, 201, fixture("pullrequest"));
    server.on_once(
        "POST",
        PR,
        400,
        json!({"type": "error", "error": {"message": "reviewers: unknown user"}}),
        &[],
    );
    let pr = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "t".into(),
                head: "a".into(),
                base: "main".into(),
                reviewers: vec!["nobody".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(pr.number, 42);
    let writes = server.writes();
    assert_eq!(writes.len(), 2);
    assert!(writes[0].json().get("reviewers").is_some());
    assert!(writes[1].json().get("reviewers").is_none());
}

#[tokio::test]
async fn a_creation_bitbucket_refuses_says_why() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        PR,
        400,
        json!({"type": "error", "error": {"message": "source: branch not found"}}),
    );
    let e = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "t".into(),
                head: "ghost".into(),
                base: "main".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("branch not found"), "{e:#}");
}

#[tokio::test]
async fn a_pull_request_merges_with_the_strategy_asked() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        &format!("{PR}/9/merge"),
        200,
        json!({"state": "MERGED"}),
    );
    for (strategy, name) in [
        (MergeStrategy::Merge, "merge_commit"),
        (MergeStrategy::Squash, "squash"),
        (MergeStrategy::Rebase, "rebase_fast_forward"),
        (MergeStrategy::SemiLinear, "rebase_merge"),
    ] {
        client(&server).merge(&repo(), 9, strategy).await.unwrap();
        assert_eq!(
            server.writes().last().unwrap().json()["merge_strategy"],
            name
        );
    }
    server.on(
        "POST",
        &format!("{PR}/10/merge"),
        409,
        json!({"type": "error", "error": {"message": "You can't merge until you resolve all merge conflicts."}}),
    );
    let e = client(&server)
        .merge(&repo(), 10, MergeStrategy::Merge)
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("merge conflicts"), "{e:#}");
}

#[tokio::test]
async fn the_checks_are_the_commit_statuses_each_key_by_its_last_word() {
    let server = FakeServer::start().await;
    server.on("GET", &format!("{PR}/42"), 200, fixture("pullrequest"));
    server.on(
        "GET",
        &format!("{PR}/42/statuses"),
        200,
        fixture("statuses"),
    );
    let checks = client(&server).checks(&repo(), 42).await.unwrap();
    let names: Vec<&str> = checks.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["Pipeline #7", "Lint", "E2E"],
        "an older status of Lint is not a check"
    );
    assert_eq!(checks[0].state, CheckState::Failure);
    assert_eq!(
        checks[0].job.as_deref(),
        Some("7"),
        "the run of Pipelines has a log"
    );
    assert!(checks[0].finished.is_some() && checks[0].started.is_some());
    assert_eq!(checks[1].state, CheckState::Success);
    assert_eq!(checks[1].job, None, "another CI keeps its logs");
    assert_eq!(
        checks[1].url.as_deref(),
        Some("https://ci.example.com/lint/55")
    );
    assert_eq!(checks[2].state, CheckState::Pending);
    assert!(checks[2].started.is_some() && checks[2].finished.is_none());
    assert!(!checks[2].cancelled);
}

#[tokio::test]
async fn a_stopped_pipeline_is_a_cancelled_failure() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{PR}/42/statuses"),
        200,
        json!({"values": [{"key": "p", "name": "Pipeline #8", "state": "STOPPED",
                           "url": "https://bitbucket.org/acme/demo/addon/pipelines/home#!/results/8"}]}),
    );
    let checks = client(&server).checks(&repo(), 42).await.unwrap();
    assert_eq!(checks[0].state, CheckState::Failure);
    assert!(checks[0].cancelled);
    assert_eq!(checks[0].job.as_deref(), Some("8"));
}

#[tokio::test]
async fn the_logs_of_a_pipelines_steps_come_one_after_the_other() {
    let server = FakeServer::start().await;
    let run = "/repositories/acme/demo/pipelines/7";
    server.on("GET", &format!("{run}/steps"), 200, fixture("steps"));
    server.on_text(
        "GET",
        &format!("{run}/steps/%7Bstep-build%7D/log"),
        200,
        BUILD_LOG,
        &[],
    );
    server.on_text(
        "GET",
        &format!("{run}/steps/%7Bstep-test%7D/log"),
        200,
        TEST_LOG,
        &[],
    );
    // The step that never ran has no log.
    server.on(
        "GET",
        &format!("{run}/steps/%7Bstep-deploy%7D/log"),
        404,
        json!({"type": "error", "error": {"message": "No log"}}),
    );
    let log = client(&server).job_log(&repo(), "7").await.unwrap();
    assert!(log.contains("=== Build (SUCCESSFUL) ==="), "{log}");
    assert!(log.contains("added 412 packages"));
    assert!(log.contains("=== Test (FAILED) ==="));
    assert!(log.contains("FAIL src/login.test.ts"));
    assert!(!log.contains("Deploy"));
    assert!(log.find("Build").unwrap() < log.find("Test").unwrap());
    assert_eq!(
        server.requests()[0].query("pagelen").as_deref(),
        Some("100")
    );

    // One step only, by its uuid.
    let one = client(&server)
        .job_log(&repo(), "7/{step-test}")
        .await
        .unwrap();
    assert_eq!(one, TEST_LOG);
}

#[tokio::test]
async fn a_very_long_log_keeps_its_end() {
    let server = FakeServer::start().await;
    let run = "/repositories/acme/demo/pipelines/7";
    let long = format!("{}THE END\n", "x".repeat(400 * 1024));
    server.on_text("GET", &format!("{run}/steps/%7Bs%7D/log"), 200, &long, &[]);
    let log = client(&server).job_log(&repo(), "7/{s}").await.unwrap();
    assert!(log.len() < 300 * 1024);
    assert!(log.ends_with("THE END\n"));
}

#[tokio::test]
async fn reviews_are_the_approvals_and_the_requests_for_changes() {
    let server = FakeServer::start().await;
    server.on("GET", &format!("{PR}/42"), 200, fixture("pullrequest"));
    let reviews = client(&server).reviews(&repo(), 42).await.unwrap();
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
}

#[tokio::test]
async fn line_comments_come_with_their_thread_and_the_conversation_beside() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{PR}/42/comments"),
        200,
        fixture("comments"),
    );
    let c = client(&server).comments(&repo(), 42).await.unwrap();
    assert_eq!(
        c.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["101", "102", "103", "104"],
        "the deleted comment is gone"
    );
    assert_eq!(c[0].path.as_deref(), Some("src/login.ts"));
    assert_eq!(c[0].line, Some(12));
    assert_eq!(c[0].author, "grace");
    assert!(!c[0].resolved && !c[1].resolved);
    assert_eq!(
        c[2].line,
        Some(3),
        "a removed line is told by its old number"
    );
    assert!(c[2].resolved);
    assert_eq!(
        (c[3].path.clone(), c[3].line, c[3].resolved),
        (None, None, false)
    );
    assert_eq!(c[3].body, "Looks good overall.");
}

#[tokio::test]
async fn the_replies_of_a_resolved_thread_are_resolved_too() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{PR}/42/comments"),
        200,
        json!({"values": [
            {"id": 1, "content": {"raw": "a"}, "user": {"nickname": "u"}, "inline": {"path": "f", "to": 1}, "parent": null,
             "resolution": {"type": "comment_resolution"}},
            {"id": 2, "content": {"raw": "b"}, "user": {"nickname": "u"}, "inline": {"path": "f", "to": 1}, "parent": {"id": 1}, "resolution": null}
        ]}),
    );
    let c = client(&server).comments(&repo(), 42).await.unwrap();
    assert!(c.iter().all(|c| c.resolved));
}

#[tokio::test]
async fn a_comment_and_an_answer_under_a_thread_are_published_and_a_thread_resolves() {
    let server = FakeServer::start().await;
    for p in ["comments", "comments/101/resolve"] {
        server.on("POST", &format!("{PR}/42/{p}"), 201, json!({"id": 200}));
    }
    let c = client(&server);
    c.comment(&repo(), 42, "Merci !").await.unwrap();
    c.reply(&repo(), 42, "101", "Fait.").await.unwrap();
    c.resolve(&repo(), 42, "101").await.unwrap();
    let w = server.writes();
    assert_eq!(w[0].json(), json!({"content": {"raw": "Merci !"}}));
    assert_eq!(
        w[1].json(),
        json!({"content": {"raw": "Fait."}, "parent": {"id": 101}})
    );
    assert_eq!(w[2].path(), format!("{PR}/42/comments/101/resolve"));
    let e = c.reply(&repo(), 42, "not-a-number", "x").await.unwrap_err();
    assert!(format!("{e:#}").contains("not-a-number"));
}

#[tokio::test]
async fn a_validated_review_is_published_with_its_line_comments_then_its_verdict() {
    let server = FakeServer::start().await;
    for p in ["comments", "approve", "request-changes"] {
        server.on("POST", &format!("{PR}/42/{p}"), 200, json!({}));
    }
    let draft = |state| ReviewDraft {
        state,
        body: "Almost.".into(),
        lines: vec![ReviewLine {
            path: "src/login.ts".into(),
            line: 12,
            body: "Constant?".into(),
        }],
    };
    let c = client(&server);
    c.review(&repo(), 42, &draft(ReviewState::ChangesRequested))
        .await
        .unwrap();
    let w = server.writes();
    assert_eq!(
        w.iter().map(|r| r.path().to_string()).collect::<Vec<_>>(),
        [
            format!("{PR}/42/comments"),
            format!("{PR}/42/comments"),
            format!("{PR}/42/request-changes")
        ]
    );
    assert_eq!(
        w[0].json(),
        json!({"content": {"raw": "Constant?"}, "inline": {"path": "src/login.ts", "to": 12}})
    );
    assert_eq!(w[1].json(), json!({"content": {"raw": "Almost."}}));

    c.review(&repo(), 42, &draft(ReviewState::Approved))
        .await
        .unwrap();
    assert_eq!(writes_to(&server, &format!("{PR}/42/approve")).len(), 1);

    // A comment alone gives no verdict.
    let before = server.writes().len();
    c.review(&repo(), 42, &draft(ReviewState::Commented))
        .await
        .unwrap();
    let after: Vec<String> = server.writes()[before..]
        .iter()
        .map(|r| r.path().to_string())
        .collect();
    assert_eq!(
        after,
        [format!("{PR}/42/comments"), format!("{PR}/42/comments")]
    );
}

#[tokio::test]
async fn deployments_come_with_their_environment_and_state() {
    let server = FakeServer::start().await;
    let base = "/repositories/acme/demo";
    server.on(
        "GET",
        &format!("{base}/refs/branches/feature%2Flogin"),
        200,
        fixture("branch"),
    );
    server.on(
        "GET",
        &format!("{base}/environments"),
        200,
        fixture("environments"),
    );
    server.on(
        "GET",
        &format!("{base}/deployments"),
        200,
        fixture("deployments"),
    );
    let d = client(&server)
        .deployments(&repo(), "feature/login")
        .await
        .unwrap();
    assert_eq!(
        d.iter()
            .map(|d| (d.environment.as_str(), d.state.as_str()))
            .collect::<Vec<_>>(),
        [("Production", "in_progress"), ("Test", "success")],
        "the deployment of another commit is not this one's"
    );
    assert_eq!(
        d[0].url.as_deref(),
        Some("https://bitbucket.org/acme/demo/pipelines/results/9")
    );

    // A hash is used as it is: the branch is not asked for.
    let before = server.requests().len();
    let d = client(&server)
        .deployments(&repo(), "a1b2c3d")
        .await
        .unwrap();
    assert_eq!(d.len(), 2);
    assert!(server.requests()[before..]
        .iter()
        .all(|r| !r.path().contains("/refs/")));
}

#[tokio::test]
async fn a_failed_or_undeployed_deployment_says_so_and_an_unknown_ref_has_none() {
    let server = FakeServer::start().await;
    let base = "/repositories/acme/demo";
    server.on(
        "GET",
        &format!("{base}/environments"),
        200,
        fixture("environments"),
    );
    server.on(
        "GET",
        &format!("{base}/deployments"),
        200,
        json!({"values": [
            {"environment": {"uuid": "{env-test}"}, "state": {"name": "COMPLETED", "status": {"name": "FAILED"}},
             "release": {"commit": {"hash": "abcdef1234567"}}},
            {"environment": {"uuid": "{env-prod}"}, "state": {"name": "UNDEPLOYED"},
             "release": {"commit": {"hash": "abcdef1234567"}}}
        ]}),
    );
    let c = client(&server);
    let d = c.deployments(&repo(), "abcdef1").await.unwrap();
    assert_eq!(
        d.iter().map(|d| d.state.as_str()).collect::<Vec<_>>(),
        ["failure", "inactive"]
    );
    // Neither a branch nor a tag: nothing to look for.
    assert!(c.deployments(&repo(), "ghost").await.unwrap().is_empty());
}

#[tokio::test]
async fn long_lists_are_followed_page_after_page_on_bitbucket_only() {
    let server = FakeServer::start().await;
    let next = format!("{}{PR}/42/comments?page=2", server.url);
    server.on(
        "GET",
        &format!("{PR}/42/comments"),
        200,
        json!({"values": [{"id": 1, "content": {"raw": "a"}, "user": {"nickname": "u"}}], "next": next}),
    );
    server.on(
        "GET",
        &format!("{PR}/42/comments?page=2"),
        200,
        json!({"values": [{"id": 2, "content": {"raw": "b"}, "user": {"nickname": "u"}}],
               "next": "https://evil.example.com/steal"}),
    );
    let c = client(&server).comments(&repo(), 42).await.unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!(
        server.requests().len(),
        2,
        "an address elsewhere is not followed with the token"
    );
}

#[tokio::test]
async fn an_expired_token_is_a_refusal_the_account_can_tell_and_says_what_bitbucket_said() {
    let server = FakeServer::start().await;
    server.on("GET", &format!("{PR}/1"), 401, fixture("error"));
    let e = client(&server).pull_request(&repo(), 1).await.unwrap_err();
    assert!(refused(&e), "{e:#}");
    assert!(format!("{e:#}").contains("Token is invalid"), "{e:#}");
}

#[tokio::test]
async fn the_environments_are_read_down_to_their_newest_success() {
    let server = FakeServer::start().await;
    let base = "/repositories/acme/demo";
    server.on(
        "GET",
        &format!("{base}/environments"),
        200,
        fixture("environments"),
    );
    server.on(
        "GET",
        &format!("{base}/deployments"),
        200,
        fixture("deployments"),
    );
    let d = client(&server).environments(&repo()).await.unwrap();
    assert_eq!(
        d.iter()
            .map(|d| (d.environment.as_str(), d.state.as_str(), d.sha.as_str()))
            .collect::<Vec<_>>(),
        [
            (
                "Production",
                "in_progress",
                "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"
            ),
            (
                "Test",
                "success",
                "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"
            ),
        ],
        "every commit's, and Test's older failure left out"
    );
    assert_eq!(d[0].url, None, "Bitbucket's environments have no address");
    assert_eq!(
        d[0].log.as_deref(),
        Some("https://bitbucket.org/acme/demo/pipelines/results/9")
    );
}

/// A branch's checks (ESC-63): the statuses of its last commit, each key by its last word; its
/// pipelines do not run again from Escouade (to verify on Bitbucket).
#[tokio::test]
async fn a_branchs_checks_are_its_last_commits_statuses() {
    let server = FakeServer::start().await;
    let base = "/repositories/acme/demo";
    server.on(
        "GET",
        &format!("{base}/refs/branches/feature%2Flogin"),
        200,
        fixture("branch"),
    );
    server.on(
        "GET",
        &format!("{base}/commit/a1b2c3d4e5f60718293a4b5c6d7e8f9012345678/statuses"),
        200,
        fixture("statuses"),
    );
    server.on(
        "GET",
        &format!("{PR}/42/statuses"),
        200,
        fixture("statuses"),
    );
    let checks = client(&server)
        .branch_checks(&repo(), "feature/login")
        .await
        .unwrap();
    assert!(!checks.is_empty());
    let pr_checks = client(&server).checks(&repo(), 42).await.unwrap();
    assert_eq!(checks, pr_checks, "read as a pull request's are");
    let e = client(&server)
        .rerun_failed_of_branch(&repo(), "feature/login")
        .await
        .unwrap_err();
    assert!(e.downcast_ref::<Unsupported>().is_some(), "{e:#}");
}

/// How a pull request may merge (ESC-62): what its target branch allows; never an auto-merge.
#[tokio::test]
async fn the_target_branch_says_how_a_pull_request_may_merge_and_there_is_no_auto_merge() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{PR}/7"),
        200,
        json!({ "id": 7, "state": "OPEN", "destination": { "branch": { "name": "main",
            "merge_strategies": ["squash", "fast_forward"], "default_merge_strategy": "squash" } } }),
    );
    let o = client(&server).merge_options(&repo(), 7).await.unwrap();
    assert_eq!(o.strategies, vec![MergeStrategy::Squash]);
    assert!(!o.auto_merge);
}
