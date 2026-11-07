//! What only the client itself shows: its pace, what it waits, what it reads of a policy, and the trial on a real organization
//! (ignored). The rest of its tests play its module through the app: `src-tauri/src/forge/azure_tests.rs`.

use super::*;
use crate::fake::FakeServer;
use serde_json::json;
use std::time::Duration;

fn connection_data() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/forges/azure/connection-data.json"
    ))
    .unwrap()
}

fn requests_to(server: &FakeServer, path: &str) -> usize {
    server
        .requests()
        .iter()
        .filter(|r| r.path() == path)
        .count()
}
#[tokio::test]
async fn a_limit_is_waited_out_then_asked_once_more() {
    let server = FakeServer::start().await;
    server.on("GET", "/acme/_apis/connectionData", 200, connection_data());
    server.on_once(
        "GET",
        "/acme/_apis/connectionData",
        429,
        json!({"message": "Request was blocked due to exceeding usage of resource."}),
        &[("retry-after", "30")],
    );
    let azure = Azure::new(
        reqwest::Client::new(),
        "https://dev.azure.com/acme".into(),
        Some(server.url.clone()),
        "pat".into(),
    )
    .with_pace(Pace {
        slow_below: 0.1,
        max_wait: Duration::from_millis(300),
    });
    let before = std::time::Instant::now();
    assert_eq!(azure.me().await.unwrap().user, "ada@acme.dev");
    let elapsed = before.elapsed();
    assert!(elapsed >= Duration::from_millis(250), "waited: {elapsed:?}");
    assert!(azure.waited() >= Duration::from_millis(250));
    assert_eq!(requests_to(&server, "/acme/_apis/connectionData"), 2);

    // A limit that holds is said after that one more try.
    let server = FakeServer::start().await;
    server.on_with(
        "GET",
        "/acme/_apis/connectionData",
        503,
        json!({"message": "Service unavailable"}),
        &[("retry-after", "1")],
    );
    let azure = Azure::new(
        reqwest::Client::new(),
        "https://dev.azure.com/acme".into(),
        Some(server.url.clone()),
        "pat".into(),
    )
    .with_pace(Pace {
        slow_below: 0.1,
        max_wait: Duration::from_millis(50),
    });
    assert!(format!("{:#}", azure.me().await.unwrap_err()).contains("503"));
    assert_eq!(requests_to(&server, "/acme/_apis/connectionData"), 2);
}
#[test]
fn a_merge_policy_says_its_strategies_its_former_setting_too() {
    assert_eq!(
        allowed_by(&json!({"allowNoFastForward": true, "allowSquash": true})),
        [MergeStrategy::Merge, MergeStrategy::Squash]
    );
    assert_eq!(
        allowed_by(&json!({"useSquashMerge": true})),
        [MergeStrategy::Squash]
    );
    assert_eq!(
        allowed_by(&json!({"useSquashMerge": false})),
        [MergeStrategy::Merge]
    );
}
// ---- A real organization ----

/// Reads and writes on a real Azure DevOps project with a PAT of the user's (Code read and
/// write, Build read and execute), on a test repository whose branch `ESCOUADE_TEST_AZURE_BRANCH`
/// is pushed and has no pull request yet:
///
/// ```text
/// ESCOUADE_TEST_AZURE_URL=https://dev.azure.com/<org> ESCOUADE_TEST_AZURE_REPO=<org>/<projet>/<dépôt>
/// ESCOUADE_TEST_AZURE_PAT=<jeton> ESCOUADE_TEST_AZURE_BRANCH=<branche> [ESCOUADE_TEST_AZURE_BASE=main]
/// cargo test --lib real_azure_devops -- --ignored --nocapture
/// ```
///
/// It opens a draft pull request, reads it back (with its checks and threads), comments, answers
/// and resolves, publishes a review without a vote, reads a failed build's log when there is one,
/// then abandons the pull request. Nothing else is touched; without its variables it does nothing.
#[tokio::test]
#[ignore = "talks to a real Azure DevOps organization"]
async fn real_azure_devops() {
    let (Ok(url), Ok(full), Ok(token), Ok(branch)) = (
        std::env::var("ESCOUADE_TEST_AZURE_URL"),
        std::env::var("ESCOUADE_TEST_AZURE_REPO"),
        std::env::var("ESCOUADE_TEST_AZURE_PAT"),
        std::env::var("ESCOUADE_TEST_AZURE_BRANCH"),
    ) else {
        return;
    };
    let base = std::env::var("ESCOUADE_TEST_AZURE_BASE").unwrap_or_else(|_| "main".into());
    let segments: Vec<&str> = full.split('/').collect();
    let [org, project, name] = segments[..] else {
        panic!("ESCOUADE_TEST_AZURE_REPO: <org>/<projet>/<dépôt>");
    };
    let repo = RepoRef {
        namespace: format!("{org}/{project}"),
        name: name.into(),
    };
    let url = url.trim_end_matches('/').to_string();
    let forge = Azure::new(reqwest::Client::new(), url, None, token);
    let me = forge.me().await.unwrap();
    println!("me: {me:?}");
    let pr = forge
        .create_pr(
            &repo,
            &PullRequestDraft {
                title: "Essai d'Escouade (ESC-241)".into(),
                body: "Ouverte et abandonnée par `real_azure_devops`.".into(),
                head: branch.clone(),
                base,
                draft: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    println!("opened: {pr:?}");
    assert!(pr.draft);
    let read = forge.pull_request(&repo, pr.number).await.unwrap();
    println!("read: {read:#?}");
    let of_branch = forge.pull_requests_of_branch(&repo, &branch).await.unwrap();
    assert_eq!(of_branch[0].number, pr.number);
    let checks = forge.checks(&repo, pr.number).await.unwrap();
    println!("checks: {checks:#?}");
    if let Some(job) = checks
        .iter()
        .find(|c| c.state == CheckState::Failure)
        .and_then(|c| c.job.clone())
    {
        let log = forge.job_log(&repo, &job).await.unwrap();
        println!("log: {}", &log[..log.len().min(400)]);
    }
    forge
        .comment(&repo, pr.number, "Commentaire d'essai.")
        .await
        .unwrap();
    forge
        .review(
            &repo,
            pr.number,
            &ReviewDraft {
                state: ReviewState::Commented,
                body: "Relecture d'essai.".into(),
                lines: vec![],
            },
        )
        .await
        .unwrap();
    let comments = forge.comments(&repo, pr.number).await.unwrap();
    println!("comments: {comments:#?}");
    if let Some(c) = comments.first() {
        forge
            .reply(&repo, pr.number, &c.id, "Réponse d'essai.")
            .await
            .unwrap();
        forge.resolve(&repo, pr.number, &c.id).await.unwrap();
    }
    println!(
        "reviews: {:?}",
        forge.reviews(&repo, pr.number).await.unwrap()
    );
    println!("rate limit: {:?}", forge.rate_limit());
    // Abandoned: the test leaves nothing open.
    forge.abandon(&repo, pr.number).await.unwrap();
    let after = forge.pull_request(&repo, pr.number).await.unwrap();
    assert_eq!(after.state, PrState::Closed);
}
