//! Azure DevOps against a fake server fed with answers shaped after Microsoft's documentation
//! (`tests/fixtures/forges/azure/`): its remotes in every form, its account (a PAT in Basic, `az`'s
//! token in Bearer), each call of its pull requests' client, and its refusals.

use super::azure::{AzureProvider, CLOUD};
use super::azure_api::{allowed_by, Azure};
use super::github_api::Pace;
use super::*;
use crate::integrations::fake::FakeServer;
use base64::Engine;
use serde_json::json;
use std::time::Duration;

fn fixture(name: &str) -> Value {
    let text = match name {
        "connection-data" => {
            include_str!("../../../tests/fixtures/forges/azure/connection-data.json")
        }
        "pull-request" => include_str!("../../../tests/fixtures/forges/azure/pull-request.json"),
        "threads" => include_str!("../../../tests/fixtures/forges/azure/threads.json"),
        "evaluations" => include_str!("../../../tests/fixtures/forges/azure/evaluations.json"),
        "statuses" => include_str!("../../../tests/fixtures/forges/azure/statuses.json"),
        "builds" => include_str!("../../../tests/fixtures/forges/azure/builds.json"),
        "timeline" => include_str!("../../../tests/fixtures/forges/azure/timeline.json"),
        "policy-configurations" => {
            include_str!("../../../tests/fixtures/forges/azure/policy-configurations.json")
        }
        _ => panic!("no fixture {name}"),
    };
    serde_json::from_str(text).unwrap()
}

const JOB_LOG: &str = include_str!("../../../tests/fixtures/forges/azure/job-log.txt");

/// What the fake server answers under, for the organization `acme` and its project `Demo Project`.
const PROJECT: &str = "/acme/Demo%20Project/_apis";
const REPO: &str = "/acme/Demo%20Project/_apis/git/repositories/demo";
const PR: &str = "/acme/Demo%20Project/_apis/git/repositories/demo/pullRequests/22";

fn repo() -> RepoRef {
    RepoRef {
        namespace: "acme/Demo Project".into(),
        name: "demo".into(),
    }
}

/// The client of the organization acme on dev.azure.com, its API being the fake server.
fn client(server: &FakeServer) -> Box<dyn Forge> {
    client_with(server, "pat-secret")
}

fn client_with(server: &FakeServer, token: &str) -> Box<dyn Forge> {
    AzureProvider.client(
        reqwest::Client::new(),
        &Credentials {
            url: "https://dev.azure.com/acme".into(),
            token: token.into(),
            api: Some(server.url.clone()),
            ..Default::default()
        },
    )
}

/// The pull request 22 and what is read with it: its threads and its checks.
fn route_pull(server: &FakeServer) {
    server.on("GET", PR, 200, fixture("pull-request"));
    server.on("GET", &format!("{PR}/threads"), 200, fixture("threads"));
    server.on("GET", &format!("{PR}/statuses"), 200, fixture("statuses"));
    server.on(
        "GET",
        &format!("{PROJECT}/policy/evaluations"),
        200,
        fixture("evaluations"),
    );
    server.on(
        "GET",
        &format!("{PROJECT}/build/builds"),
        200,
        fixture("builds"),
    );
}

fn requests_to(server: &FakeServer, path: &str) -> usize {
    server
        .requests()
        .iter()
        .filter(|r| r.path() == path)
        .count()
}

fn analyze(url: &str) -> Option<(Option<String>, String, String, String)> {
    Registry::standard()
        .analyze(url, |_| None)
        .map(|r| (r.kind, r.host, r.url, r.repo.full()))
}

fn azure(url: &str, repo: &str) -> Option<(Option<String>, String, String, String)> {
    Some((Some("azure".into()), CLOUD.into(), url.into(), repo.into()))
}

// ---- Remotes and the account ----

#[test]
fn dev_azure_com_remotes_are_an_organization_a_project_and_a_repository() {
    for url in [
        "https://dev.azure.com/acme/Demo%20Project/_git/demo",
        "https://acme@dev.azure.com/acme/Demo%20Project/_git/demo",
        "https://dev.azure.com/acme/Demo%20Project/_git/demo/",
        "git@ssh.dev.azure.com:v3/acme/Demo%20Project/demo",
        "ssh://git@ssh.dev.azure.com/v3/acme/Demo%20Project/demo",
        // The former addresses of the same organization.
        "https://acme.visualstudio.com/Demo%20Project/_git/demo",
        "https://acme.visualstudio.com/DefaultCollection/Demo%20Project/_git/demo",
        "acme@vs-ssh.visualstudio.com:v3/acme/Demo%20Project/demo",
    ] {
        assert_eq!(
            analyze(url),
            azure("https://dev.azure.com/acme", "acme/Demo Project/demo"),
            "{url}"
        );
    }
    // A repository bearing its project's name: the project is left out of the path.
    assert_eq!(
        analyze("https://dev.azure.com/acme/_git/demo"),
        azure("https://dev.azure.com/acme", "acme/demo/demo")
    );
    assert_eq!(
        analyze("https://acme.visualstudio.com/_git/demo"),
        azure("https://dev.azure.com/acme", "acme/demo/demo")
    );
    // Not a repository: a page of it, or an organization alone.
    assert_eq!(
        analyze("https://dev.azure.com/acme/Demo/_git/demo/pullrequest/22"),
        None
    );
    assert_eq!(analyze("https://dev.azure.com/acme"), None);
    assert_eq!(analyze("git@ssh.dev.azure.com:v3/acme/demo"), None);
}

#[test]
fn a_server_is_known_by_its_account_and_reads_its_collection() {
    let known = |host: &str| (host == "tfs.acme.dev").then(|| "azure".to_string());
    let read = |url: &str| {
        Registry::standard()
            .analyze(url, known)
            .map(|r| (r.kind, r.host, r.repo.full()))
    };
    let want = Some((
        Some("azure".to_string()),
        "tfs.acme.dev".to_string(),
        "DefaultCollection/Demo Project/demo".to_string(),
    ));
    for url in [
        "https://tfs.acme.dev/tfs/DefaultCollection/Demo%20Project/_git/demo",
        "https://tfs.acme.dev:8443/tfs/DefaultCollection/Demo%20Project/_git/demo",
        "ssh://tfs.acme.dev:22/tfs/DefaultCollection/Demo%20Project/_git/demo",
        "ssh://tfs.acme.dev:22/DefaultCollection/Demo%20Project/_git/demo",
    ] {
        assert_eq!(read(url), want, "{url}");
    }
    // Without its account, a Server is a host Escouade does not know.
    let r = Registry::standard()
        .analyze(
            "https://tfs.acme.dev/tfs/DefaultCollection/Demo/_git/demo",
            |_| None,
        )
        .unwrap();
    assert_eq!(r.kind, None);
}

#[test]
fn the_account_is_an_organizations_and_its_token_page_and_cli_are_azures() {
    let p = AzureProvider;
    // The former address of an organization is kept as dev.azure.com's.
    assert_eq!(
        p.instance("https://acme.visualstudio.com".into()),
        "https://dev.azure.com/acme"
    );
    assert_eq!(
        p.instance("https://dev.azure.com/acme".into()),
        "https://dev.azure.com/acme"
    );
    assert_eq!(
        p.instance("https://tfs.acme.dev/tfs/DefaultCollection".into()),
        "https://tfs.acme.dev/tfs/DefaultCollection"
    );
    assert_eq!(
        p.token_page("https://dev.azure.com/acme"),
        "https://dev.azure.com/acme/_usersSettings/tokens"
    );
    assert_eq!(
        p.scopes(),
        ["vso.code_write", "vso.build_execute", "vso.work_write"]
    );
    let az = p.cli(CLOUD).unwrap();
    assert_eq!(az.program, "az");
    assert_eq!(az.login, "az login");
    assert!(az
        .args
        .windows(2)
        .any(|w| w == ["--resource", "499b84ac-1321-427f-aa17-267ca6975798"]));
    assert_eq!(
        az.parse("eyJ0.eyJ1.c2ln\r\n", ""),
        Some("eyJ0.eyJ1.c2ln".into())
    );
    assert_eq!(az.parse("", "ERROR: Please run 'az login'"), None);
    // A Server knows no Entra ID token.
    assert!(p.cli("tfs.acme.dev").is_none());
    let caps = p.capabilities();
    assert!(caps.pull_requests && caps.draft && caps.auto_merge && caps.rerun);
    assert!(
        !caps.deployments,
        "nothing simple says a branch's deployments"
    );
    assert_eq!(
        caps.merge_strategies,
        [
            MergeStrategy::Merge,
            MergeStrategy::Squash,
            MergeStrategy::Rebase,
            MergeStrategy::SemiLinear
        ]
    );
    assert_eq!(p.request_word(), "PR");
}

#[tokio::test]
async fn a_personal_token_goes_in_basic_and_says_who_it_is() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/acme/_apis/connectionData",
        200,
        fixture("connection-data"),
    );
    let me = client(&server).me().await.unwrap();
    assert_eq!(me.user, "ada@acme.dev");
    assert_eq!(me.label, "ada@acme.dev");
    assert_eq!(me.scopes, None, "Azure does not tell a token's rights");
    let auth = server.requests()[0].headers["authorization"].clone();
    let basic = base64::engine::general_purpose::STANDARD.encode(":pat-secret");
    assert_eq!(
        auth,
        format!("Basic {basic}"),
        "an empty user, then the PAT"
    );
}

#[tokio::test]
async fn the_az_clis_token_goes_in_bearer() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/acme/_apis/connectionData",
        200,
        fixture("connection-data"),
    );
    let jwt = "eyJ0eXAiOiJKV1QifQ.eyJleHAiOjF9.c2lnbmF0dXJl";
    client_with(&server, jwt).me().await.unwrap();
    assert_eq!(
        server.requests()[0].headers["authorization"],
        format!("Bearer {jwt}")
    );
}

#[tokio::test]
async fn an_account_without_an_organization_reads_the_profile() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/_apis/profile/profiles/me",
        200,
        json!({"displayName": "Ada Lovelace", "publicAlias": "6f7e", "emailAddress": "ada@acme.dev", "id": "6f7e"}),
    );
    let forge = AzureProvider.client(
        reqwest::Client::new(),
        &Credentials {
            url: "https://dev.azure.com".into(),
            token: "pat".into(),
            api: Some(server.url.clone()),
            ..Default::default()
        },
    );
    assert_eq!(forge.me().await.unwrap().label, "ada@acme.dev");
    assert_eq!(
        server.requests()[0].query("api-version").as_deref(),
        Some("7.1")
    );
}

#[tokio::test]
async fn a_refused_token_is_a_refusal_whatever_azure_answers() {
    for (status, body) in [
        (
            401,
            json!({"message": "TF400813: The user is not authorized to access this resource."}),
        ),
        // A token Azure does not take: its sign-in page.
        (203, json!("<html>Sign in</html>")),
        // An anonymous user: no token went through.
        (
            200,
            json!({"authenticatedUser": {"id": "aaaa", "descriptor": "Microsoft.TeamFoundation.UnauthenticatedIdentity;Anonymous", "providerDisplayName": "Anonymous"}}),
        ),
    ] {
        let server = FakeServer::start().await;
        server.on("GET", "/acme/_apis/connectionData", status, body);
        let e = client(&server).me().await.unwrap_err();
        assert!(refused(&e), "{status}: {e:#}");
    }
    // The same sign-in page after a redirect, a 200 in HTML.
    let server = FakeServer::start().await;
    server.on_text(
        "GET",
        PR,
        200,
        "<!DOCTYPE html><html>Azure DevOps Services | Sign In</html>",
        &[],
    );
    let e = client(&server).pull_request(&repo(), 22).await.unwrap_err();
    assert!(refused(&e), "{e:#}");
}

#[tokio::test]
async fn a_denied_or_missing_resource_says_what_azure_said() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        PR,
        403,
        json!({"$id": "1", "message": "TF401027: You need the Git 'PullRequestContribute' permission.", "typeKey": "GitNeedsPermissionException"}),
    );
    let e = client(&server).pull_request(&repo(), 22).await.unwrap_err();
    let said = format!("{e:#}");
    assert!(said.contains("403") && said.contains("TF401027"), "{said}");
    assert!(!refused(&e));
    let server = FakeServer::start().await;
    server.on(
        "GET",
        &format!("{REPO}/pullRequests/9"),
        404,
        json!({"message": "TF401180: The requested pull request was not found."}),
    );
    let e = client(&server).pull_request(&repo(), 9).await.unwrap_err();
    assert!(format!("{e:#}").contains("404"), "{e:#}");
}

#[tokio::test]
async fn a_limit_is_waited_out_then_asked_once_more() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/acme/_apis/connectionData",
        200,
        fixture("connection-data"),
    );
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

#[tokio::test]
async fn the_limits_headers_are_kept() {
    let server = FakeServer::start().await;
    server.on_with(
        "GET",
        "/acme/_apis/connectionData",
        200,
        fixture("connection-data"),
        &[
            ("x-ratelimit-resource", "ATCPU"),
            ("x-ratelimit-limit", "200"),
            ("x-ratelimit-remaining", "150"),
            ("x-ratelimit-reset", "1798000000"),
        ],
    );
    let forge = client(&server);
    forge.me().await.unwrap();
    assert_eq!(
        forge.rate_limit(),
        Some(RateLimit {
            limit: 200,
            remaining: 150,
            reset: 1798000000
        })
    );
}

// ---- Pull requests ----

#[tokio::test]
async fn a_pull_request_is_read_with_its_votes_threads_and_checks() {
    let server = FakeServer::start().await;
    route_pull(&server);
    let pr = client(&server).pull_request(&repo(), 22).await.unwrap();
    assert_eq!(pr.number, 22);
    assert_eq!(
        pr.url,
        "https://dev.azure.com/acme/Demo%20Project/_git/demo/pullrequest/22"
    );
    assert_eq!(pr.title, "Connexion par jeton");
    assert_eq!(pr.state, PrState::Open);
    assert_eq!((pr.head.as_str(), pr.base.as_str()), ("feat/login", "main"));
    assert!(!pr.draft);
    assert_eq!(pr.mergeable, Some(true));
    // Grace approved; the team's vote is hers again; Bob waits for the author.
    assert_eq!(pr.approvals, 1);
    assert!(pr.changes_requested);
    // One thread on a line still active (another fixed, one deleted, one not on a line).
    assert_eq!(pr.open_threads, 1);
    assert!(!pr.auto_merge);
    assert!(!pr.fork);
    assert_eq!(pr.checks, Some(CheckState::Failure));
    for r in server.requests() {
        let want = if r.path().ends_with("/policy/evaluations") {
            "7.1-preview.1"
        } else {
            "7.1"
        };
        assert_eq!(
            r.query("api-version").as_deref(),
            Some(want),
            "{}",
            r.target
        );
    }
    // The evaluations hang from the project's id and the pull request's.
    let eval = server
        .requests()
        .into_iter()
        .find(|r| r.path().ends_with("/policy/evaluations"))
        .unwrap();
    assert_eq!(
        eval.query("artifactId").as_deref(),
        Some("vstfs:///CodeReview/CodeReviewId/7c3e4d55-6b1a-4c2a-9f7f-2b3e4d5f6a7b/22")
    );
}

#[tokio::test]
async fn conflicts_drafts_merged_and_abandoned_ones_are_told() {
    let server = FakeServer::start().await;
    let mut p = fixture("pull-request");
    p["mergeStatus"] = json!("conflicts");
    p["isDraft"] = json!(true);
    p["autoCompleteSetBy"] = json!({"id": "6f7e"});
    p["forkSource"] = json!({"repository": {"name": "demo-fork"}});
    route_pull(&server);
    server.on("GET", PR, 200, p.clone());
    let pr = client(&server).pull_request(&repo(), 22).await.unwrap();
    assert_eq!(pr.mergeable, Some(false));
    assert!(pr.draft && pr.auto_merge && pr.fork);

    // Completed: merged, and nothing more asked.
    let server = FakeServer::start().await;
    p["status"] = json!("completed");
    server.on("GET", PR, 200, p.clone());
    let pr = client(&server).pull_request(&repo(), 22).await.unwrap();
    assert_eq!(pr.state, PrState::Merged);
    assert_eq!(pr.checks, None);
    assert_eq!(server.requests().len(), 1);
    let server = FakeServer::start().await;
    p["status"] = json!("abandoned");
    server.on("GET", PR, 200, p);
    assert_eq!(
        client(&server)
            .pull_request(&repo(), 22)
            .await
            .unwrap()
            .state,
        PrState::Closed
    );
}

#[tokio::test]
async fn a_branchs_pull_requests_are_found_by_its_ref_newest_first() {
    let server = FakeServer::start().await;
    route_pull(&server);
    let mut old = fixture("pull-request");
    old["pullRequestId"] = json!(7);
    old["status"] = json!("abandoned");
    let new = fixture("pull-request");
    server.on(
        "GET",
        &format!("{REPO}/pullrequests"),
        200,
        json!({"value": [old, new], "count": 2}),
    );
    let prs = client(&server)
        .pull_requests_of_branch(&repo(), "feat/login")
        .await
        .unwrap();
    assert_eq!(prs.iter().map(|p| p.number).collect::<Vec<_>>(), [22, 7]);
    let asked = server
        .requests()
        .into_iter()
        .find(|r| r.path() == format!("{REPO}/pullrequests"))
        .unwrap();
    assert_eq!(
        asked.query("searchCriteria.sourceRefName").as_deref(),
        Some("refs/heads/feat/login")
    );
    assert_eq!(asked.query("searchCriteria.status").as_deref(), Some("all"));
}

#[tokio::test]
async fn several_pull_requests_are_read_and_one_gone_is_left_out() {
    let server = FakeServer::start().await;
    route_pull(&server);
    server.on(
        "GET",
        &format!("{REPO}/pullRequests/9"),
        404,
        json!({"message": "TF401180: The requested pull request was not found."}),
    );
    let prs = client(&server)
        .pull_requests(&repo(), &[22, 9])
        .await
        .unwrap();
    assert_eq!(prs.iter().map(|p| p.number).collect::<Vec<_>>(), [22]);
}

#[tokio::test]
async fn a_pull_request_opens_as_a_draft_with_its_reviewers_labels_and_work_item() {
    let server = FakeServer::start().await;
    let mut made = fixture("pull-request");
    made["isDraft"] = json!(true);
    made["pullRequestId"] = json!(23);
    server.on("POST", &format!("{REPO}/pullrequests"), 201, made);
    server.on(
        "GET",
        "/acme/_apis/identities",
        200,
        json!({"count": 1, "value": [{"id": "d6245f20-2af8-44f4-9451-8107cb2767db", "providerDisplayName": "Grace Hopper"}]}),
    );
    server.on(
        "GET",
        "/acme/_apis/identities?searchFilter=General&filterValue=nobody%40acme.dev&queryMembership=None&api-version=7.1",
        200,
        json!({"count": 0, "value": []}),
    );
    let at = format!("{REPO}/pullRequests/23");
    server.on(
        "PUT",
        &format!("{at}/reviewers/d6245f20-2af8-44f4-9451-8107cb2767db"),
        200,
        json!({"vote": 0}),
    );
    server.on(
        "POST",
        &format!("{at}/labels"),
        200,
        json!({"name": "escouade"}),
    );
    let long = "x".repeat(5000);
    let pr = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "Connexion par jeton".into(),
                body: long,
                head: "feat/login".into(),
                base: "main".into(),
                draft: true,
                reviewers: vec!["grace@acme.dev".into(), "nobody@acme.dev".into()],
                labels: vec!["escouade".into()],
                closes: Some(42),
            },
        )
        .await
        .unwrap();
    assert_eq!(pr.number, 23);
    assert!(pr.draft);
    let writes = server.writes();
    let body = writes[0].json();
    assert_eq!(body["sourceRefName"], "refs/heads/feat/login");
    assert_eq!(body["targetRefName"], "refs/heads/main");
    assert_eq!(body["isDraft"], true);
    assert_eq!(body["workItemRefs"], json!([{"id": "42"}]));
    // Azure takes 4000 characters at most.
    assert_eq!(body["description"].as_str().unwrap().chars().count(), 4000);
    // The reviewer found is added, the one Azure does not know is left out; the label follows.
    let paths: Vec<String> = writes.iter().map(|w| w.path().to_string()).collect();
    assert_eq!(
        paths,
        [
            format!("{REPO}/pullrequests"),
            format!("{at}/reviewers/d6245f20-2af8-44f4-9451-8107cb2767db"),
            format!("{at}/labels"),
        ]
    );
}

#[tokio::test]
async fn a_refused_reviewer_does_not_take_the_pull_request_back() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        &format!("{REPO}/pullrequests"),
        201,
        fixture("pull-request"),
    );
    server.on(
        "GET",
        "/acme/_apis/identities",
        200,
        json!({"value": [{"id": "1f2e"}]}),
    );
    server.on(
        "PUT",
        &format!("{PR}/reviewers/1f2e"),
        400,
        json!({"message": "TF401176: The identity is not a member."}),
    );
    let pr = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "t".into(),
                head: "feat/login".into(),
                base: "main".into(),
                reviewers: vec!["bob".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(pr.number, 22);
}

#[tokio::test]
async fn a_pull_request_merges_with_a_strategy_its_branch_allows() {
    let server = FakeServer::start().await;
    server.on("GET", PR, 200, fixture("pull-request"));
    server.on(
        "GET",
        &format!("{PROJECT}/git/policy/configurations"),
        200,
        fixture("policy-configurations"),
    );
    server.on("PATCH", PR, 200, fixture("pull-request"));
    let forge = client(&server);
    forge
        .merge(&repo(), 22, MergeStrategy::Squash)
        .await
        .unwrap();
    forge
        .merge(&repo(), 22, MergeStrategy::SemiLinear)
        .await
        .unwrap();
    let bodies: Vec<Value> = server.writes().iter().map(|w| w.json()).collect();
    assert_eq!(bodies[0]["status"], "completed");
    assert_eq!(
        bodies[0]["lastMergeSourceCommit"]["commitId"],
        "b60280bc6e62e2f880f1b63c1e24987664d3bda3"
    );
    assert_eq!(bodies[0]["completionOptions"]["mergeStrategy"], "squash");
    assert_eq!(
        bodies[1]["completionOptions"]["mergeStrategy"],
        "rebaseMerge"
    );
    let policy = server
        .requests()
        .into_iter()
        .find(|r| r.path().ends_with("/git/policy/configurations"))
        .unwrap();
    assert_eq!(policy.query("refName").as_deref(), Some("refs/heads/main"));
    assert_eq!(
        policy.query("repositoryId").as_deref(),
        Some("3411ebc1-d5aa-464f-9615-0b527bc66719")
    );

    // A merge commit the branch does not allow: said, nothing sent.
    let e = forge
        .merge(&repo(), 22, MergeStrategy::Merge)
        .await
        .unwrap_err();
    let said = format!("{e:#}");
    assert!(said.contains("squash, semi-linear"), "{said}");
    assert_eq!(server.writes().len(), 2);
}

#[tokio::test]
async fn without_a_merge_policy_every_strategy_is_asked() {
    let server = FakeServer::start().await;
    server.on("GET", PR, 200, fixture("pull-request"));
    server.on(
        "GET",
        &format!("{PROJECT}/git/policy/configurations"),
        200,
        json!({"value": [], "count": 0}),
    );
    server.on("PATCH", PR, 200, fixture("pull-request"));
    client(&server)
        .merge(&repo(), 22, MergeStrategy::Rebase)
        .await
        .unwrap();
    assert_eq!(
        server.writes()[0].json()["completionOptions"]["mergeStrategy"],
        "rebase"
    );
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

#[tokio::test]
async fn auto_complete_is_set_by_the_tokens_identity() {
    let server = FakeServer::start().await;
    server.on("GET", PR, 200, fixture("pull-request"));
    server.on(
        "GET",
        &format!("{PROJECT}/git/policy/configurations"),
        200,
        fixture("policy-configurations"),
    );
    server.on(
        "GET",
        "/acme/_apis/connectionData",
        200,
        fixture("connection-data"),
    );
    let mut set = fixture("pull-request");
    set["autoCompleteSetBy"] = json!({"id": "6f7e6b4f-1c2d-4a3b-9e8f-0a1b2c3d4e5f"});
    server.on("PATCH", PR, 200, set);
    client(&server)
        .auto_merge(&repo(), 22, MergeStrategy::Squash)
        .await
        .unwrap();
    let body = server.writes()[0].json();
    assert_eq!(
        body["autoCompleteSetBy"]["id"],
        "6f7e6b4f-1c2d-4a3b-9e8f-0a1b2c3d4e5f"
    );
    assert_eq!(body["completionOptions"]["mergeStrategy"], "squash");

    // Azure answered without it: said.
    let server2 = FakeServer::start().await;
    server2.on("GET", PR, 200, fixture("pull-request"));
    server2.on(
        "GET",
        "/acme/_apis/connectionData",
        200,
        fixture("connection-data"),
    );
    server2.on("PATCH", PR, 200, fixture("pull-request"));
    assert!(client(&server2)
        .auto_merge(&repo(), 22, MergeStrategy::Squash)
        .await
        .is_err());
}

#[tokio::test]
async fn checks_are_the_policies_the_statuses_and_the_builds_each_once() {
    let server = FakeServer::start().await;
    route_pull(&server);
    let checks = client(&server).checks(&repo(), 22).await.unwrap();
    let seen: Vec<(&str, CheckState, Option<&str>)> = checks
        .iter()
        .map(|c| (c.name.as_str(), c.state, c.job.as_deref()))
        .collect();
    assert_eq!(
        seen,
        [
            // The build policy, its build 501 failed.
            ("CI", CheckState::Failure, Some("501")),
            // The status policy, the same as its status: once.
            ("quality/sonar", CheckState::Success, None),
            ("licenses", CheckState::Failure, None),
            // The pipelines no policy holds: the newest build of each.
            ("e2e", CheckState::Success, Some("502")),
            ("lint", CheckState::Pending, Some("503")),
        ]
    );
    assert_eq!(
        checks[0].url.as_deref(),
        Some(
            format!(
                "{}/acme/Demo%20Project/_build/results?buildId=501",
                server.url
            )
            .as_str()
        )
    );
    assert_eq!(
        checks[2].url.as_deref(),
        Some("https://licenses.acme.dev/demo/22")
    );
    let builds = server
        .requests()
        .into_iter()
        .find(|r| r.path().ends_with("/build/builds"))
        .unwrap();
    assert_eq!(
        builds.query("branchName").as_deref(),
        Some("refs/pull/22/merge")
    );
}

#[tokio::test]
async fn a_builds_log_is_its_failed_tasks() {
    let server = FakeServer::start().await;
    let build = format!("{PROJECT}/build/builds/501");
    server.on(
        "GET",
        &format!("{build}/timeline"),
        200,
        fixture("timeline"),
    );
    server.on_text(
        "GET",
        &format!("{build}/logs/12"),
        200,
        JOB_LOG,
        &[("content-type", "text/plain")],
    );
    let log = client(&server).job_log(&repo(), "501").await.unwrap();
    assert!(log.starts_with("##[section]npm test\n"), "{log}");
    assert!(log.contains("FAIL src/login.test.ts"), "{log}");
    let asked = server
        .requests()
        .into_iter()
        .find(|r| r.path().ends_with("/logs/12"))
        .unwrap();
    assert_eq!(asked.headers["accept"], "text/plain");
    assert_eq!(requests_to(&server, &format!("{build}/logs/3")), 0);
}

#[tokio::test]
async fn failed_builds_are_retried_once_each() {
    let server = FakeServer::start().await;
    route_pull(&server);
    server.on(
        "PATCH",
        &format!("{PROJECT}/build/builds/501"),
        200,
        json!({"id": 501, "status": "notStarted"}),
    );
    let n = client(&server).rerun_failed(&repo(), 22).await.unwrap();
    assert_eq!(n, 1, "the licenses' status has no build to run again");
    let w = &server.writes()[0];
    assert_eq!(w.query("retry").as_deref(), Some("true"));

    // Nothing failed: nothing runs.
    let server = FakeServer::start().await;
    route_pull(&server);
    server.on("GET", &format!("{PR}/statuses"), 200, json!({"value": []}));
    server.on(
        "GET",
        &format!("{PROJECT}/policy/evaluations"),
        200,
        json!({"value": []}),
    );
    server.on(
        "GET",
        &format!("{PROJECT}/build/builds"),
        200,
        json!({"value": [fixture("builds")["value"][0]]}),
    );
    assert_eq!(client(&server).rerun_failed(&repo(), 22).await.unwrap(), 0);
    assert!(server.writes().is_empty());
}

#[tokio::test]
async fn reviews_are_the_reviewers_votes() {
    let server = FakeServer::start().await;
    server.on("GET", PR, 200, fixture("pull-request"));
    let reviews = client(&server).reviews(&repo(), 22).await.unwrap();
    assert_eq!(
        reviews
            .iter()
            .map(|r| (r.author.as_str(), r.state))
            .collect::<Vec<_>>(),
        [
            ("Grace Hopper", ReviewState::Approved),
            ("Bob Martin", ReviewState::ChangesRequested)
        ]
    );
}

#[tokio::test]
async fn comments_come_from_the_threads_people_wrote() {
    let server = FakeServer::start().await;
    server.on("GET", &format!("{PR}/threads"), 200, fixture("threads"));
    let comments = client(&server).comments(&repo(), 22).await.unwrap();
    type Seen<'a> = (&'a str, &'a str, Option<&'a str>, Option<u32>, bool);
    let seen: Vec<Seen> = comments
        .iter()
        .map(|c| {
            (
                c.id.as_str(),
                c.author.as_str(),
                c.path.as_deref(),
                c.line,
                c.resolved,
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("148:1", "Bob Martin", Some("src/login.ts"), Some(12), false),
            (
                "148:2",
                "Ada Lovelace",
                Some("src/login.ts"),
                Some(12),
                false
            ),
            (
                "150:1",
                "Grace Hopper",
                Some("src/login.test.ts"),
                Some(3),
                true
            ),
            ("151:1", "Grace Hopper", None, None, false),
        ]
    );
    assert_eq!(comments[0].body, "Ce nom est peu clair.");
}

#[tokio::test]
async fn an_answer_a_resolution_and_a_comment_are_published() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        &format!("{PR}/threads/148/comments"),
        200,
        json!({"id": 3}),
    );
    server.on(
        "PATCH",
        &format!("{PR}/threads/148"),
        200,
        json!({"id": 148, "status": "fixed"}),
    );
    server.on("POST", &format!("{PR}/threads"), 200, json!({"id": 153}));
    let forge = client(&server);
    forge
        .reply(&repo(), 22, "148:1", "C'est renommé.")
        .await
        .unwrap();
    forge.resolve(&repo(), 22, "148:1").await.unwrap();
    forge.comment(&repo(), 22, "Prêt.").await.unwrap();
    let w = server.writes();
    assert_eq!(
        w[0].json(),
        json!({"content": "C'est renommé.", "parentCommentId": 1, "commentType": 1})
    );
    assert_eq!(w[1].json(), json!({"status": "fixed"}));
    assert_eq!(w[2].json()["comments"][0]["content"], "Prêt.");
    assert!(forge.reply(&repo(), 22, "148", "x").await.is_err());
}

#[tokio::test]
async fn a_validated_review_posts_its_threads_then_votes() {
    let server = FakeServer::start().await;
    server.on("POST", &format!("{PR}/threads"), 200, json!({"id": 160}));
    server.on(
        "GET",
        "/acme/_apis/connectionData",
        200,
        fixture("connection-data"),
    );
    let me = format!("{PR}/reviewers/6f7e6b4f-1c2d-4a3b-9e8f-0a1b2c3d4e5f");
    server.on("PUT", &me, 200, json!({"vote": -5}));
    client(&server)
        .review(
            &repo(),
            22,
            &ReviewDraft {
                state: ReviewState::ChangesRequested,
                body: "Deux points.".into(),
                lines: vec![ReviewLine {
                    path: "src/login.ts".into(),
                    line: 12,
                    body: "Renomme.".into(),
                }],
            },
        )
        .await
        .unwrap();
    let w = server.writes();
    assert_eq!(w.len(), 3);
    let line = w[0].json();
    assert_eq!(line["threadContext"]["filePath"], "/src/login.ts");
    assert_eq!(line["threadContext"]["rightFileStart"]["line"], 12);
    assert_eq!(line["comments"][0]["content"], "Renomme.");
    assert_eq!(w[1].json()["comments"][0]["content"], "Deux points.");
    assert_eq!(
        (w[2].path(), w[2].json()),
        (me.as_str(), json!({"vote": -5}))
    );

    // A comment alone votes nothing.
    let server = FakeServer::start().await;
    server.on("POST", &format!("{PR}/threads"), 200, json!({"id": 161}));
    client(&server)
        .review(
            &repo(),
            22,
            &ReviewDraft {
                state: ReviewState::Commented,
                body: "Vu.".into(),
                lines: vec![],
            },
        )
        .await
        .unwrap();
    assert_eq!(server.writes().len(), 1);
}

#[tokio::test]
async fn a_server_is_reached_under_its_collection() {
    let server = FakeServer::start().await;
    let base = "/tfs/DefaultCollection/Demo%20Project/_apis/git/repositories/demo";
    server.on(
        "GET",
        "/tfs/DefaultCollection/_apis/connectionData",
        200,
        fixture("connection-data"),
    );
    server.on(
        "GET",
        &format!("{base}/pullrequests"),
        200,
        json!({"value": []}),
    );
    let forge = AzureProvider.client(
        reqwest::Client::new(),
        &Credentials {
            url: format!("{}/tfs/DefaultCollection", server.url),
            token: "pat".into(),
            ..Default::default()
        },
    );
    assert_eq!(forge.me().await.unwrap().label, "ada@acme.dev");
    let repo = RepoRef {
        namespace: "DefaultCollection/Demo Project".into(),
        name: "demo".into(),
    };
    assert!(forge
        .pull_requests_of_branch(&repo, "feat/login")
        .await
        .unwrap()
        .is_empty());
    assert_eq!(requests_to(&server, &format!("{base}/pullrequests")), 1);
}

#[test]
fn an_entra_id_token_is_asked_again_before_it_ends() {
    use super::connect::expires_soon;
    let jwt = |exp: u64| {
        let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        format!(
            "{}.{}.sig",
            e.encode(br#"{"typ":"JWT"}"#),
            e.encode(format!(r#"{{"exp":{exp}}}"#))
        )
    };
    assert!(!expires_soon(&jwt(10_000), 1_000));
    assert!(expires_soon(&jwt(1_200), 1_000), "within five minutes");
    assert!(expires_soon(&jwt(500), 1_000));
    // A personal token never expires here.
    assert!(!expires_soon("pat-52-characters", 1_000));
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
    let forge = AzureProvider.client(
        reqwest::Client::new(),
        &Credentials {
            url: AzureProvider.instance(instance_url(&url)),
            token,
            ..Default::default()
        },
    );
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
    let azure = Azure::new(
        reqwest::Client::new(),
        AzureProvider.instance(instance_url(&url)),
        None,
        std::env::var("ESCOUADE_TEST_AZURE_PAT").unwrap(),
    );
    azure.abandon(&repo, pr.number).await.unwrap();
    let after = forge.pull_request(&repo, pr.number).await.unwrap();
    assert_eq!(after.state, PrState::Closed);
}
