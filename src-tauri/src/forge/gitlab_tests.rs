//! GitLab's client (the plugin « GitLab », `plugins/gitlab`, ESC-56) against a fake server fed
//! with answers recorded from gitlab.com (`tests/fixtures/forges/gitlab/`, see `record.mjs` and its README): a merged
//! merge request of a project in a subgroup, opened from a fork three groups deep, with its approvals, reviewers,
//! pipeline, jobs and a job's log; its discussions and the deployments are written from GitLab's documentation (gitlab.com
//! refuses them without a token), and small made-up answers stand for what the recording does not hold (a conflict, a
//! failed job, a limit).
//!
//! An account's calls go to the plugin's module by `ProcessForge`, as the core makes them,
//! and the plugin calls the fake server: on gitlab.com's account (its API said, as the core says a fake one) and on a
//! self-managed instance's (its API found from its address, `/api/v4`). What only the client itself shows (its pace) is
//! read on it directly.

use super::*;
use crate::integrations::fake::FakeServer;
use serde_json::json;

fn fixture(name: &str) -> Value {
    let text = match name {
        "project" => include_str!("../../../tests/fixtures/forges/gitlab/project.json"),
        "fork-project" => include_str!("../../../tests/fixtures/forges/gitlab/fork-project.json"),
        "merge-request" => include_str!("../../../tests/fixtures/forges/gitlab/merge-request.json"),
        "branch-mrs" => include_str!("../../../tests/fixtures/forges/gitlab/branch-mrs.json"),
        "approvals" => include_str!("../../../tests/fixtures/forges/gitlab/approvals.json"),
        "reviewers" => include_str!("../../../tests/fixtures/forges/gitlab/reviewers.json"),
        "jobs" => include_str!("../../../tests/fixtures/forges/gitlab/jobs.json"),
        "discussions" => include_str!("../../../tests/fixtures/forges/gitlab/discussions.json"),
        "deployments" => include_str!("../../../tests/fixtures/forges/gitlab/deployments.json"),
        _ => panic!("no fixture {name}"),
    };
    serde_json::from_str(text).unwrap()
}

const JOB_LOG: &str = include_str!("../../../tests/fixtures/forges/gitlab/job-log.txt");

/// `gitlab-org/api/client-go`, as its address names it: one segment.
const P: &str = "/projects/gitlab-org%2Fapi%2Fclient-go";
const MR: &str = "/projects/gitlab-org%2Fapi%2Fclient-go/merge_requests/3101";
/// The project of the merge request, and its head pipeline.
const JOBS: &str = "/projects/65271576/pipelines/3004244757/jobs";

fn repo() -> RepoRef {
    RepoRef {
        namespace: "gitlab-org/api".into(),
        name: "client-go".into(),
    }
}

/// The client of gitlab.com's account, its API being the fake server.
fn client(server: &FakeServer) -> Box<dyn Forge> {
    served(&Credentials {
        url: "https://gitlab.com".into(),
        token: "glpat-test".into(),
        api: Some(server.url.clone()),
        ..Default::default()
    })
}

/// The client of a self-managed instance's account: the fake server is the instance, its API at `/api/v4`.
fn instance(server: &FakeServer) -> Box<dyn Forge> {
    served(&Credentials {
        url: server.url.clone(),
        token: "glpat-own".into(),
        ..Default::default()
    })
}

fn served(creds: &Credentials) -> Box<dyn Forge> {
    crate::plugins::builtin::served_forge(crate::plugins::builtin::GITLAB, "gitlab", creds)
}

fn route_recorded_mr(server: &FakeServer) {
    server.on("GET", MR, 200, fixture("merge-request"));
    server.on("GET", &format!("{MR}/approvals"), 200, fixture("approvals"));
    server.on("GET", "/projects/65275361", 200, fixture("fork-project"));
}

fn requests_to(server: &FakeServer, path: &str) -> usize {
    server
        .requests()
        .iter()
        .filter(|r| r.path() == path)
        .count()
}

/// The recorded merge request, opened again: what an open one with its say says.
fn open_mr(changes: Value) -> Value {
    let mut m = fixture("merge-request");
    m["state"] = "opened".into();
    m["merged_at"] = Value::Null;
    m["merge_when_pipeline_succeeds"] = false.into();
    m["detailed_merge_status"] = "mergeable".into();
    for (k, v) in changes.as_object().unwrap() {
        m[k] = v.clone();
    }
    m
}

#[tokio::test]
async fn a_recorded_merge_request_of_a_subgroup_is_read_with_its_approvals_and_its_fork() {
    let server = FakeServer::start().await;
    route_recorded_mr(&server);
    let pr = client(&server).pull_request(&repo(), 3101).await.unwrap();
    assert_eq!(pr.number, 3101);
    assert_eq!(pr.state, PrState::Merged);
    assert_eq!(
        pr.url,
        "https://gitlab.com/gitlab-org/api/client-go/-/merge_requests/3101"
    );
    assert_eq!(pr.title, "feat(merge_requests): add CancelAutoMerge");
    assert_eq!(
        (pr.head.as_str(), pr.base.as_str()),
        ("cancel-auto-merge", "main")
    );
    assert!(!pr.draft);
    assert_eq!(pr.mergeable, None, "nothing said of a merged one");
    assert_eq!(pr.approvals, 2);
    assert!(!pr.changes_requested);
    assert_eq!(pr.open_threads, 0);
    assert!(!pr.auto_merge, "merged: nothing left to merge by itself");
    assert_eq!(pr.checks, Some(CheckState::Success));
    assert_eq!(pr.author, "frank");
    assert!(pr.created > 0);
    assert!(pr.fork);
    assert_eq!(pr.head_repo, "gitlab-community/gitlab-org/api/client-go");
    // Every thread resolved, as GitLab says: the discussions are not read.
    assert_eq!(requests_to(&server, &format!("{MR}/discussions")), 0);
    assert_eq!(server.requests().len(), 3);
    assert!(server
        .requests()
        .iter()
        .all(|r| r.headers["private-token"] == "glpat-test"));
}

#[tokio::test]
async fn a_forks_path_is_read_once_by_an_account() {
    let server = FakeServer::start().await;
    route_recorded_mr(&server);
    let c = client(&server);
    c.pull_request(&repo(), 3101).await.unwrap();
    c.pull_requests(&repo(), &[3101]).await.unwrap();
    assert_eq!(requests_to(&server, "/projects/65275361"), 1);
    assert_eq!(requests_to(&server, MR), 2);
}

#[tokio::test]
async fn on_a_self_managed_instance_an_open_merge_request_says_its_conflict_draft_and_threads() {
    let server = FakeServer::start().await;
    let own = "/api/v4/projects/acme%2Fweb%2Ffront%2Fapp/merge_requests/7";
    server.on(
        "GET",
        own,
        200,
        open_mr(json!({
            "iid": 7,
            "title": "Draft: Fix the login",
            "draft": true,
            "has_conflicts": true,
            "detailed_merge_status": "conflict",
            "blocking_discussions_resolved": false,
            "source_project_id": 65271576,
            "merge_when_pipeline_succeeds": true,
            "head_pipeline": {"id": 9, "project_id": 65271576, "status": "running"},
        })),
    );
    server.on(
        "GET",
        &format!("{own}/approvals"),
        200,
        json!({"approved_by": []}),
    );
    server.on(
        "GET",
        &format!("{own}/discussions"),
        200,
        fixture("discussions"),
    );
    let repo = RepoRef {
        namespace: "acme/web/front".into(),
        name: "app".into(),
    };
    let pr = instance(&server).pull_request(&repo, 7).await.unwrap();
    assert_eq!(pr.title, "Fix the login", "the draft's mark is the flag");
    assert!(pr.draft);
    assert_eq!(pr.mergeable, Some(false), "a conflict");
    assert_eq!(pr.open_threads, 1, "one thread of three left open");
    assert!(pr.auto_merge);
    assert_eq!(pr.checks, Some(CheckState::Pending));
    assert_eq!(pr.approvals, 0);
    assert!(!pr.fork);
    assert!(server
        .requests()
        .iter()
        .all(|r| r.headers["private-token"] == "glpat-own" && r.path().starts_with("/api/v4/")));
}

#[tokio::test]
async fn changes_asked_say_it_and_a_merge_request_being_checked_says_nothing_of_its_conflict() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        MR,
        200,
        open_mr(
            json!({"detailed_merge_status": "requested_changes", "source_project_id": 65271576}),
        ),
    );
    server.on("GET", &format!("{MR}/approvals"), 404, Value::Null);
    let pr = client(&server).pull_request(&repo(), 3101).await.unwrap();
    assert!(pr.changes_requested);
    assert_eq!(pr.mergeable, Some(true));
    assert_eq!(pr.approvals, 0, "approvals refused: none");

    server.on(
        "GET",
        MR,
        200,
        open_mr(json!({"detailed_merge_status": "checking", "source_project_id": 65271576})),
    );
    let pr = client(&server).pull_request(&repo(), 3101).await.unwrap();
    assert_eq!(pr.mergeable, None);
}

#[tokio::test]
async fn a_merge_request_that_does_not_exist_is_an_error_and_is_left_out_of_a_list() {
    let server = FakeServer::start().await;
    route_recorded_mr(&server);
    let err = client(&server)
        .pull_request(&repo(), 9)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("!9"), "{err}");
    let prs = client(&server)
        .pull_requests(&repo(), &[3101, 9])
        .await
        .unwrap();
    assert_eq!(prs.iter().map(|p| p.number).collect::<Vec<_>>(), [3101]);
}

#[tokio::test]
async fn a_branchs_merge_requests_are_found_by_its_name() {
    let server = FakeServer::start().await;
    route_recorded_mr(&server);
    server.on(
        "GET",
        &format!("{P}/merge_requests"),
        200,
        fixture("branch-mrs"),
    );
    let prs = client(&server)
        .pull_requests_of_branch(&repo(), "cancel-auto-merge")
        .await
        .unwrap();
    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].number, 3101);
    assert_eq!(prs[0].checks, Some(CheckState::Success));
    let list = server
        .requests()
        .into_iter()
        .find(|r| r.path() == format!("{P}/merge_requests"))
        .unwrap();
    assert_eq!(
        list.query("source_branch").as_deref(),
        Some("cancel-auto-merge")
    );
}

#[tokio::test]
async fn the_open_merge_requests_cost_one_call() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{P}/merge_requests"),
        200,
        json!([
            open_mr(json!({"iid": 3164, "reviewers": [{"username": "ada"}, {"username": "bob"}]})),
            open_mr(json!({"iid": 3165, "draft": true, "reviewers": []}))
        ]),
    );
    let prs = client(&server).open_pull_requests(&repo()).await.unwrap();
    assert_eq!(
        prs.iter().map(|p| p.number).collect::<Vec<_>>(),
        [3164, 3165]
    );
    assert!(prs[1].draft);
    // Whose review is asked (« Relecture demandée », ESC-65).
    assert_eq!(prs[0].reviewers, ["ada", "bob"]);
    assert!(prs[1].reviewers.is_empty());
    let sent = server.requests();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].query("state").as_deref(), Some("opened"));
    assert_eq!(sent[0].query("per_page").as_deref(), Some("30"));
}

#[tokio::test]
async fn a_forks_parent_is_read_with_its_subgroups() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/projects/gitlab-community%2Fgitlab-org%2Fapi%2Fclient-go",
        200,
        fixture("fork-project"),
    );
    server.on("GET", P, 200, fixture("project"));
    let fork = RepoRef {
        namespace: "gitlab-community/gitlab-org/api".into(),
        name: "client-go".into(),
    };
    assert_eq!(
        client(&server).parent(&fork).await.unwrap(),
        Some(repo()),
        "groups and subgroups kept"
    );
    assert_eq!(client(&server).parent(&repo()).await.unwrap(), None);
}

#[tokio::test]
async fn a_merge_request_is_opened_as_a_draft_with_its_reviewers_labels_and_the_issue_it_closes() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        &format!("{P}/merge_requests"),
        201,
        open_mr(json!({"iid": 3200, "title": "Draft: Fix the login", "draft": true})),
    );
    server.on(
        "GET",
        "/users?username=alice",
        200,
        json!([{"id": 3714742, "username": "alice"}]),
    );
    server.on("GET", "/users?username=nobody", 200, json!([]));
    let made = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "Fix the login".into(),
                body: "What it does.".into(),
                head: "esc-12".into(),
                base: "main".into(),
                draft: true,
                reviewers: vec!["@alice".into(), "nobody".into()],
                labels: vec!["backend".into(), "type::bug".into()],
                closes: Some(12),
            },
        )
        .await
        .unwrap();
    assert_eq!(made.number, 3200);
    assert!(made.draft);
    let sent = server.writes();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0].json(),
        json!({
            "source_branch": "esc-12",
            "target_branch": "main",
            "title": "Draft: Fix the login",
            "description": "What it does.\n\nCloses #12",
            "labels": "backend,type::bug",
            "reviewer_ids": [3714742],
        })
    );
}

#[tokio::test]
async fn a_merge_request_merges_or_squashes_and_refuses_a_rebase_it_cannot_choose() {
    let server = FakeServer::start().await;
    server.on("PUT", &format!("{MR}/merge"), 200, fixture("merge-request"));
    let c = client(&server);
    c.merge(&repo(), 3101, MergeStrategy::Squash).await.unwrap();
    c.merge(&repo(), 3101, MergeStrategy::Merge).await.unwrap();
    let bodies: Vec<Value> = server.writes().iter().map(|r| r.json()).collect();
    assert_eq!(bodies, [json!({"squash": true}), json!({"squash": false})]);
    assert!(c.merge(&repo(), 3101, MergeStrategy::Rebase).await.is_err());
    assert_eq!(server.writes().len(), 2, "nothing sent for a rebase");
}

#[tokio::test]
async fn auto_merge_is_asked_in_the_words_of_every_version() {
    let server = FakeServer::start().await;
    server.on(
        "PUT",
        &format!("{MR}/merge"),
        200,
        open_mr(json!({"merge_when_pipeline_succeeds": true})),
    );
    client(&server)
        .auto_merge(&repo(), 3101, MergeStrategy::Squash)
        .await
        .unwrap();
    assert_eq!(
        server.writes()[0].json(),
        json!({"squash": true, "auto_merge": true, "merge_when_pipeline_succeeds": true})
    );
    // GitLab took the call but did not set it.
    server.on("PUT", &format!("{MR}/merge"), 200, open_mr(json!({})));
    assert!(client(&server)
        .auto_merge(&repo(), 3101, MergeStrategy::Merge)
        .await
        .is_err());
    // Already merged or not mergeable: GitLab's 405 says it.
    server.on(
        "PUT",
        &format!("{MR}/merge"),
        405,
        json!({"message": "405 Method Not Allowed"}),
    );
    let err = client(&server)
        .auto_merge(&repo(), 3101, MergeStrategy::Merge)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("405"), "{err}");
}

#[tokio::test]
async fn the_checks_are_the_recorded_jobs_of_the_head_pipeline() {
    let server = FakeServer::start().await;
    server.on("GET", MR, 200, fixture("merge-request"));
    server.on("GET", JOBS, 200, fixture("jobs"));
    let checks = client(&server).checks(&repo(), 3101).await.unwrap();
    assert_eq!(checks.len(), 12);
    let by = |name: &str| checks.iter().find(|c| c.name == name).unwrap();
    let it = by("tests:integration");
    assert_eq!(it.state, CheckState::Success);
    assert_eq!(it.job.as_deref(), Some("17577084459"));
    assert!(it.started.is_some() && it.finished > it.started);
    assert!(it.url.as_deref().unwrap().contains("/-/jobs/17577084459"));
    // Allowed to fail: GitLab's warning, the pipeline passes.
    assert_eq!(by("danger-review").state, CheckState::Success);
    assert!(checks.iter().all(|c| !c.cancelled));
}

#[tokio::test]
async fn a_merge_request_without_pipeline_has_no_check_and_nothing_to_run_again() {
    let server = FakeServer::start().await;
    server.on("GET", MR, 200, open_mr(json!({"head_pipeline": null})));
    assert!(client(&server)
        .checks(&repo(), 3101)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        client(&server).rerun_failed(&repo(), 3101).await.unwrap(),
        0
    );
    assert!(server.writes().is_empty());
}

#[tokio::test]
async fn a_jobs_log_is_its_trace_in_its_own_project() {
    let server = FakeServer::start().await;
    server.on_text(
        "GET",
        &format!("{P}/jobs/17577084459/trace"),
        200,
        JOB_LOG,
        &[],
    );
    server.on_text("GET", "/projects/42/jobs/7/trace", 200, "fork's log", &[]);
    let log = client(&server)
        .job_log(&repo(), "17577084459")
        .await
        .unwrap();
    assert!(log.contains("Running with gitlab-runner"), "{log}");
    assert_eq!(
        client(&server).job_log(&repo(), "42:7").await.unwrap(),
        "fork's log"
    );
}

#[tokio::test]
async fn the_failed_jobs_run_again_in_one_retry_of_their_pipeline() {
    let server = FakeServer::start().await;
    server.on("GET", MR, 200, fixture("merge-request"));
    server.on("GET", JOBS, 200, fixture("jobs"));
    server.on(
        "POST",
        "/projects/65271576/pipelines/3004244757/retry",
        201,
        json!({"id": 3004244757u64, "status": "pending"}),
    );
    // The recorded pipeline passed (its two failures were allowed): nothing to run.
    assert_eq!(
        client(&server).rerun_failed(&repo(), 3101).await.unwrap(),
        0
    );
    assert!(server.writes().is_empty());

    let mut jobs = fixture("jobs");
    jobs[0]["status"] = "failed".into();
    jobs[1]["status"] = "failed".into();
    server.on("GET", JOBS, 200, jobs);
    assert_eq!(
        client(&server).rerun_failed(&repo(), 3101).await.unwrap(),
        1
    );
    let sent = server.writes();
    assert_eq!(sent.len(), 1, "one call for both");
    assert_eq!(
        sent[0].path(),
        "/projects/65271576/pipelines/3004244757/retry"
    );
}

#[tokio::test]
async fn the_reviews_are_the_approvals_and_the_reviewers_say() {
    let server = FakeServer::start().await;
    server.on("GET", &format!("{MR}/approvals"), 200, fixture("approvals"));
    server.on("GET", &format!("{MR}/reviewers"), 200, fixture("reviewers"));
    let reviews = client(&server).reviews(&repo(), 3101).await.unwrap();
    assert_eq!(
        reviews
            .iter()
            .map(|r| (r.author.as_str(), r.state))
            .collect::<Vec<_>>(),
        [
            ("alice", ReviewState::Approved),
            ("bob", ReviewState::Approved)
        ]
    );

    let mut reviewers = fixture("reviewers");
    reviewers[1]["state"] = "requested_changes".into();
    reviewers
        .as_array_mut()
        .unwrap()
        .push(json!({"user": {"username": "ada"}, "state": "reviewed"}));
    server.on("GET", &format!("{MR}/reviewers"), 200, reviewers);
    let mut approvals = fixture("approvals");
    approvals["approved_by"].as_array_mut().unwrap().truncate(1);
    server.on("GET", &format!("{MR}/approvals"), 200, approvals);
    let reviews = client(&server).reviews(&repo(), 3101).await.unwrap();
    assert_eq!(
        reviews
            .iter()
            .map(|r| (r.author.as_str(), r.state))
            .collect::<Vec<_>>(),
        [
            ("alice", ReviewState::Approved),
            ("bob", ReviewState::ChangesRequested),
            ("ada", ReviewState::Commented)
        ]
    );
    // An instance without `/reviewers`: the approvals alone.
    server.on("GET", &format!("{MR}/reviewers"), 404, Value::Null);
    assert_eq!(
        client(&server).reviews(&repo(), 3101).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn the_discussions_are_the_comments_on_lines_and_beside_them_system_notes_aside() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{MR}/discussions"),
        200,
        fixture("discussions"),
    );
    let comments = client(&server).comments(&repo(), 3101).await.unwrap();
    assert_eq!(comments.len(), 4, "the system note left out");
    assert_eq!(
        comments[0].id,
        "6a9c1750b37d513a43987b574953fceb50b03ce7:2829102301"
    );
    assert_eq!(comments[0].author, "alice");
    assert_eq!(comments[0].path.as_deref(), Some("merge_requests.go"));
    assert_eq!(comments[0].line, Some(1412));
    assert!(comments[0].resolved);
    let open = &comments[2];
    assert_eq!(
        (open.path.as_deref(), open.line, open.resolved),
        (Some("merge_requests_test.go"), Some(880), false),
        "a removed line's"
    );
    assert_eq!(
        (comments[3].path.as_deref(), comments[3].line),
        (None, None)
    );
}

#[tokio::test]
async fn a_comment_an_answer_and_a_resolution_go_to_their_place() {
    let server = FakeServer::start().await;
    let thread = "87805b7c09016a7058e91bdbe7b29d1f284a39e6";
    server.on("POST", &format!("{MR}/notes"), 201, json!({"id": 1}));
    server.on(
        "POST",
        &format!("{MR}/discussions/{thread}/notes"),
        201,
        json!({"id": 2}),
    );
    server.on(
        "PUT",
        &format!("{MR}/discussions/{thread}"),
        200,
        json!({"id": thread}),
    );
    let c = client(&server);
    c.comment(&repo(), 3101, "Fixed in the last commit.")
        .await
        .unwrap();
    c.reply(&repo(), 3101, &format!("{thread}:2831008764"), "Done.")
        .await
        .unwrap();
    c.resolve(&repo(), 3101, &format!("{thread}:2831008764"))
        .await
        .unwrap();
    let sent: Vec<(String, String, Value)> = server
        .writes()
        .iter()
        .map(|r| (r.method.clone(), r.path().to_string(), r.json()))
        .collect();
    assert_eq!(
        sent,
        [
            (
                "POST".to_string(),
                format!("{MR}/notes"),
                json!({"body": "Fixed in the last commit."})
            ),
            (
                "POST".to_string(),
                format!("{MR}/discussions/{thread}/notes"),
                json!({"body": "Done."})
            ),
            (
                "PUT".to_string(),
                format!("{MR}/discussions/{thread}"),
                json!({"resolved": true})
            ),
        ]
    );
}

#[tokio::test]
async fn a_validated_review_publishes_its_lines_on_the_diff_its_text_then_its_approval() {
    let server = FakeServer::start().await;
    server.on("GET", MR, 200, fixture("merge-request"));
    server.on(
        "POST",
        &format!("{MR}/discussions"),
        201,
        json!({"id": "d"}),
    );
    server.on("POST", &format!("{MR}/notes"), 201, json!({"id": 1}));
    server.on("POST", &format!("{MR}/approve"), 201, json!({}));
    client(&server)
        .review(
            &repo(),
            3101,
            &ReviewDraft {
                state: ReviewState::Approved,
                body: "Looks good.".into(),
                lines: vec![ReviewLine {
                    path: "merge_requests.go".into(),
                    line: 1412,
                    body: "Nit: a doc comment.".into(),
                }],
            },
        )
        .await
        .unwrap();
    let sent = server.writes();
    assert_eq!(
        sent.iter()
            .map(|r| r.path().to_string())
            .collect::<Vec<_>>(),
        [
            format!("{MR}/discussions"),
            format!("{MR}/notes"),
            format!("{MR}/approve")
        ]
    );
    assert_eq!(
        sent[0].json()["position"],
        json!({
            "position_type": "text",
            "base_sha": "ea8d5daf0c65b923f6efd885e0adb9e41187353b",
            "start_sha": "ea8d5daf0c65b923f6efd885e0adb9e41187353b",
            "head_sha": "24c5c32eb4f7dae8f76fb33a02204ab46509d63c",
            "old_path": "merge_requests.go",
            "new_path": "merge_requests.go",
            "new_line": 1412,
        })
    );
    assert_eq!(sent[1].json(), json!({"body": "Looks good."}));
}

#[tokio::test]
async fn the_deployments_of_a_branch_are_the_last_of_each_environment() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{P}/deployments"),
        200,
        fixture("deployments"),
    );
    let deps = client(&server)
        .deployments(&repo(), "cancel-auto-merge")
        .await
        .unwrap();
    let seen: Vec<(&str, &str, Option<&str>)> = deps
        .iter()
        .map(|d| (d.environment.as_str(), d.state.as_str(), d.url.as_deref()))
        .collect();
    assert_eq!(
        seen,
        [
            (
                "docs-preview",
                "in_progress",
                Some("https://gitlab.com/gitlab-org/api/client-go/-/jobs/17577084480")
            ),
            (
                "review/cancel-auto-merge",
                "success",
                Some("https://cancel-auto-merge.review.example.dev")
            ),
        ],
        "the older failure of the review and main's production left out"
    );
    assert_eq!(deps[0].sha, "24c5c32eb4f7dae8f76fb33a02204ab46509d63c");
    assert_eq!(deps[0].author, "frank");
    assert!(deps[0].created > 0);
    assert_eq!(
        server.requests()[0].query("order_by").as_deref(),
        Some("updated_at")
    );
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[tokio::test]
async fn the_environments_are_read_down_to_their_newest_success() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{P}/deployments"),
        200,
        fixture("deployments"),
    );
    let deps = client(&server).environments(&repo()).await.unwrap();
    let seen: Vec<(&str, &str)> = deps
        .iter()
        .map(|d| (d.environment.as_str(), d.state.as_str()))
        .collect();
    assert_eq!(
        seen,
        [
            ("docs-preview", "in_progress"),
            ("review/cancel-auto-merge", "success"),
            ("production", "success"),
        ],
        "whatever their branch, and the review's older failure left out"
    );
    // The environment's address alone: a job's goes in its log.
    assert_eq!(deps[0].url, None);
    assert_eq!(
        deps[0].log.as_deref(),
        Some("https://gitlab.com/gitlab-org/api/client-go/-/jobs/17577084480")
    );
}

/// A branch's checks (ESC-63): the jobs of its last pipeline; failed, retried in one call.
#[tokio::test]
async fn a_branchs_checks_are_its_last_pipelines_jobs_retried_in_one_call() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{P}/pipelines"),
        200,
        json!([{ "id": 3004244757u64, "project_id": 65271576, "ref": "main", "status": "failed" }]),
    );
    let mut jobs = fixture("jobs");
    jobs[0]["status"] = "failed".into();
    server.on("GET", JOBS, 200, jobs);
    server.on(
        "POST",
        "/projects/65271576/pipelines/3004244757/retry",
        201,
        json!({"id": 3004244757u64, "status": "pending"}),
    );
    let checks = client(&server)
        .branch_checks(&repo(), "main")
        .await
        .unwrap();
    assert!(!checks.is_empty());
    assert_eq!(checks[0].state, CheckState::Failure);
    assert!(
        checks
            .iter()
            .all(|c| c.job.as_deref().is_some_and(|j| !j.contains(':'))),
        "jobs of the project itself"
    );
    let asked = server
        .requests()
        .into_iter()
        .find(|r| r.path() == format!("{P}/pipelines"))
        .unwrap();
    assert_eq!(asked.query("ref").as_deref(), Some("main"));
    assert_eq!(
        client(&server)
            .rerun_failed_of_branch(&repo(), "main")
            .await
            .unwrap(),
        1
    );
    // A branch without a pipeline: no check, nothing to retry.
    server.on("GET", &format!("{P}/pipelines"), 200, json!([]));
    assert!(client(&server)
        .branch_checks(&repo(), "main")
        .await
        .unwrap()
        .is_empty());
}

/// How a merge request may merge (ESC-62): `squash_option` says whether a merge commit or a
/// squash is allowed; GitLab's auto-merge is always there.
#[tokio::test]
async fn what_gitlab_says_of_its_limit_reaches_escouade() {
    let server = FakeServer::start().await;
    let reset = (now_secs() + 60).to_string();
    server.on_with(
        "GET",
        MR,
        200,
        fixture("merge-request"),
        &[
            ("ratelimit-limit", "2000"),
            ("ratelimit-remaining", "1990"),
            ("ratelimit-reset", &reset),
        ],
    );
    server.on("GET", &format!("{MR}/approvals"), 200, fixture("approvals"));
    server.on("GET", "/projects/65275361", 200, fixture("fork-project"));
    let c = client(&server);
    c.pull_request(&repo(), 3101).await.unwrap();
    assert_eq!(
        c.rate_limit().map(|l| (l.limit, l.remaining)),
        Some((2000, 1990)),
        "the account's budget, from the plugin's answer"
    );
}

#[tokio::test]
async fn the_projects_squash_option_says_how_a_merge_request_may_merge() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/projects/gitlab-org%2Fapi%2Fclient-go",
        200,
        json!({ "id": 1, "squash_option": "always", "merge_method": "merge" }),
    );
    let o = client(&server).merge_options(&repo(), 7).await.unwrap();
    assert_eq!(o.strategies, vec![MergeStrategy::Squash]);
    assert!(o.auto_merge);
}
