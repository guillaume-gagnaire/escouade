//! The pull requests of the tickets (ESC-59): what they open with, the page without an account,
//! their reading and what it does to the ticket, against a fake GitHub served by its plugin.

use super::*;
use crate::agentprs_tests::github_project;
use crate::core_tests::{git, harness, Harness};
use crate::i18n::Lang::{En, Fr};
use crate::integrations::fake::FakeServer;
use crate::model::{ExternalRef, Service, Ticket};
use crate::tickets_tests::reviewed;
use serde_json::{json, Value};

fn repo(full: &str) -> RepoRef {
    let (namespace, name) = full.rsplit_once('/').unwrap();
    RepoRef {
        namespace: namespace.into(),
        name: name.into(),
    }
}

fn pr(number: u64, state: PrState) -> PullRequest {
    PullRequest {
        number,
        url: format!("https://github.com/acme/demo/pull/{number}"),
        state,
        base: "main".into(),
        head: "ticket/dem-1".into(),
        ..Default::default()
    }
}

fn in_review(number: u64) -> Ticket {
    Ticket {
        id: "t1".into(),
        key: "DEM-1".into(),
        column: Column::Pr,
        pr: Some(ticket_pr(
            &pr(number, PrState::Open),
            "github",
            "github.com",
            &repo("acme/demo"),
            "PR",
            "main",
            "ticket/dem-1",
            1,
        )),
        ..Default::default()
    }
}

fn check(state: CheckState) -> Check {
    Check {
        name: "ci".into(),
        state,
        ..Default::default()
    }
}

#[test]
fn the_issue_of_the_same_repository_is_closed_by_the_pull_request() {
    let mut t = Ticket::default();
    let from = |service: Service, id: &str, container: &str| ExternalRef {
        service,
        id: id.into(),
        key: format!("#{id}"),
        container: container.into(),
        ..Default::default()
    };
    assert_eq!(closes_issue(&t, "github", &repo("acme/demo")), None);
    t.external = Some(from(Service::GITHUB, "12", "Acme/Demo"));
    assert_eq!(closes_issue(&t, "github", &repo("acme/demo")), Some(12));
    // Another repository, another host: its issue is not this pull request's to close.
    assert_eq!(closes_issue(&t, "github", &repo("acme/other")), None);
    assert_eq!(closes_issue(&t, "gitlab", &repo("acme/demo")), None);
    t.external = Some(from(Service::GITLAB, "7", "group/sub/demo"));
    assert_eq!(closes_issue(&t, "gitlab", &repo("group/sub/demo")), Some(7));
    // Jira's key is in the title already; Azure Boards' work item goes by `link_work_item`.
    t.external = Some(from(Service::JIRA, "ATL-3", "acme/demo"));
    assert_eq!(closes_issue(&t, "github", &repo("acme/demo")), None);
}

#[test]
fn without_an_account_each_host_opens_its_page_filled_in() {
    let page = |kind: &str, url: &str, full: &str| {
        creation_page(
            kind,
            url,
            &repo(full),
            "main",
            "ticket/dem-1",
            "feat: x [DEM-1]",
            "Corps",
        )
    };
    assert_eq!(
        page("github", "https://github.com", "acme/demo").unwrap(),
        "https://github.com/acme/demo/compare/main...ticket/dem-1?expand=1&title=feat%3A%20x%20%5BDEM-1%5D&body=Corps"
    );
    assert_eq!(
        page("gitea", "https://codeberg.org/", "ada/demo").unwrap(),
        "https://codeberg.org/ada/demo/compare/main...ticket/dem-1?expand=1&title=feat%3A%20x%20%5BDEM-1%5D&body=Corps"
    );
    assert_eq!(
        page("gitlab", "https://gitlab.com", "group/sub/demo").unwrap(),
        "https://gitlab.com/group/sub/demo/-/merge_requests/new?merge_request%5Bsource_branch%5D=ticket%2Fdem-1&merge_request%5Btarget_branch%5D=main&merge_request%5Btitle%5D=feat%3A%20x%20%5BDEM-1%5D&merge_request%5Bdescription%5D=Corps"
    );
    assert_eq!(
        page("bitbucket", "https://bitbucket.org", "acme/demo").unwrap(),
        "https://bitbucket.org/acme/demo/pull-requests/new?source=ticket%2Fdem-1&dest=main"
    );
    assert_eq!(
        page("azure", "https://dev.azure.com/acme", "Web Shop/web").unwrap(),
        "https://dev.azure.com/acme/Web%20Shop/_git/web/pullrequestcreate?sourceRef=ticket%2Fdem-1&targetRef=main"
    );
    assert_eq!(page("unknown", "https://git.acme.dev", "a/b"), None);
    // A long description does not make an address too long.
    let long = "é".repeat(20_000);
    let url = creation_page(
        "gitlab",
        "https://gitlab.com",
        &repo("a/b"),
        "main",
        "x",
        &long,
        &long,
    )
    .unwrap();
    assert!(url.len() < 6 * 1024, "{}", url.len());
}

#[test]
fn a_reading_shows_the_state_the_ci_the_approvals_and_the_conflict_on_the_card() {
    let mut t = in_review(12);
    let mut read = pr(12, PrState::Open);
    read.draft = true;
    read.checks = Some(CheckState::Failure);
    read.approvals = 1;
    read.changes_requested = true;
    read.open_threads = 2;
    read.mergeable = Some(false);
    let checks = [
        check(CheckState::Success),
        check(CheckState::Failure),
        check(CheckState::Skipped),
    ];
    assert_eq!(settle(&mut t, &read, Some(&checks), 50, Fr), News::Same);
    let tp = t.pr.clone().unwrap();
    assert_eq!(t.column, Column::Pr);
    assert!(tp.draft && tp.changes_requested);
    assert_eq!(tp.checks, Some(CheckState::Failure));
    assert_eq!((tp.checks_total, tp.checks_failed), (2, 1));
    assert_eq!((tp.approvals, tp.open_threads), (1, 2));
    assert_eq!(tp.mergeable, Some(false));
    assert_eq!(tp.checked_at, Some(50));
    // The checks not read this time: the counts stay.
    settle(&mut t, &read, None, 60, Fr);
    assert_eq!(t.pr.as_ref().unwrap().checks_total, 2);
}

#[test]
fn an_error_of_the_host_is_said_and_never_moves_the_ticket() {
    let mut t = in_review(12);
    let tp = t.pr.as_mut().unwrap();
    failed(tp, "GitHub injoignable : délai dépassé".into(), 100);
    failed(tp, "GitHub injoignable : délai dépassé".into(), 200);
    assert_eq!(tp.error_since, Some(100), "since its first failure");
    assert_eq!(t.column, Column::Pr);
    assert_eq!(t.blocked, None);
    // Read well again: the error goes.
    settle(&mut t, &pr(12, PrState::Open), None, 300, Fr);
    assert_eq!(t.pr.as_ref().unwrap().error, None);
    assert_eq!(t.pr.as_ref().unwrap().error_since, None);
    let e = anyhow!(crate::integrations::HttpError {
        status: 401,
        message: "Bad credentials".into(),
    });
    assert_eq!(
        error_text(Fr, "GitHub", &e),
        "GitHub : jeton refusé ou expiré, reconnecte le compte"
    );
    assert_eq!(
        error_text(En, "GitLab", &anyhow!("connection refused")),
        "GitLab out of reach: connection refused"
    );
}

#[test]
fn merged_on_the_host_the_ticket_is_done_closed_it_is_blocked() {
    let mut t = in_review(12);
    assert_eq!(
        settle(&mut t, &pr(12, PrState::Merged), None, 70, En),
        News::Merged
    );
    assert_eq!(t.column, Column::Done);
    assert_eq!(t.done_at, Some(70));
    assert_eq!(
        t.outcome_claude.as_deref(),
        Some("⤵ PR #12 merged into main")
    );
    assert_eq!(
        t.outcome_url.as_deref(),
        Some("https://github.com/acme/demo/pull/12")
    );
    // Done: a later reading changes nothing.
    assert_eq!(
        settle(&mut t, &pr(12, PrState::Closed), None, 80, En),
        News::Same
    );
    assert_eq!(t.column, Column::Done);

    let mut t = in_review(12);
    t.pr.as_mut().unwrap().word = "MR".into();
    assert_eq!(
        settle(&mut t, &pr(12, PrState::Closed), None, 70, En),
        News::Closed
    );
    assert_eq!(t.column, Column::Pr);
    assert!(t.pr.as_ref().unwrap().closed);
    assert_eq!(t.blocked.as_deref(), Some("MR #12 fermée sans merge"));
    // Said once.
    assert_eq!(
        settle(&mut t, &pr(12, PrState::Closed), None, 80, En),
        News::Same
    );
    // Opened again on the host: followed again.
    assert_eq!(
        settle(&mut t, &pr(12, PrState::Open), None, 90, En),
        News::Same
    );
    assert!(!t.pr.as_ref().unwrap().closed);
    assert_eq!(t.blocked, None);
}

#[test]
fn the_followed_ones_are_grouped_by_repository_and_read_every_minute_in_front() {
    let mut a = in_review(1);
    let mut b = in_review(2);
    b.id = "t2".into();
    let mut closed = in_review(3);
    closed.id = "t3".into();
    closed.pr.as_mut().unwrap().closed = true;
    let mut done = in_review(4);
    done.id = "t4".into();
    done.column = Column::Done;
    a.project_id = "p".into();
    b.project_id = "p".into();
    let all = [a, b, closed, done];
    let groups = followed(&all, None);
    assert_eq!(groups.len(), 1);
    let list = groups.values().next().unwrap();
    assert_eq!(list, &vec![("t1".to_string(), 1), ("t2".to_string(), 2)]);
    assert!(followed(&all, Some("other")).is_empty());
    assert_eq!(every(true), Duration::from_secs(60));
    assert_eq!(every(false), Duration::from_secs(300));
}

// ---------- On the real core, against a fake GitHub ----------

fn graphql_pr(number: u64, state: &str, rollup: &str) -> Value {
    json!({
        "number": number, "url": format!("https://github.com/acme/demo/pull/{number}"),
        "title": "feat", "state": state, "isDraft": false,
        "headRefName": "ticket/dem-1", "baseRefName": "main", "mergeable": "CONFLICTING",
        "isCrossRepository": false, "autoMergeRequest": null,
        "latestOpinionatedReviews": {"nodes": [{"state": "APPROVED"}]},
        "reviewThreads": {"nodes": [{"isResolved": false}, {"isResolved": true}]},
        "commits": {"nodes": [{"commit": {"statusCheckRollup": {"state": rollup}}}]},
    })
}

fn answer_pulls(server: &FakeServer, number: u64, state: &str) {
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        json!({ "data": { "repository": { format!("p{number}"): graphql_pr(number, state, "SUCCESS") } } }),
    );
}

fn has_branch(repo: &std::path::Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "rev-parse",
            "--verify",
            "-q",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .unwrap()
        .success()
}

async fn board_on_github(
    h: &Harness,
    server: &FakeServer,
) -> (crate::model::Project, std::path::PathBuf) {
    let (p, r, _) = github_project(h, server, false).await;
    h.set_board(&p.id, |s| {
        s.action = "pr".into();
        s.draft = true;
        s.reviewers = vec!["grace".into()];
        s.labels = vec!["escouade".into()];
    });
    (p, r)
}

/// « Valider + PR » opens it by the API (draft, reviewers, labels, the issue it closes), the
/// ticket waits « En revue »; merged on the host, it is done and the one after it starts.
#[tokio::test]
async fn a_tickets_pull_request_opens_by_the_api_and_its_merge_ends_the_ticket() {
    let h = harness("tp-merge");
    let server = FakeServer::start().await;
    let (p, r) = board_on_github(&h, &server).await;
    server.on(
        "POST",
        "/repos/acme/demo/pulls",
        201,
        json!({
            "number": 12, "html_url": "https://github.com/acme/demo/pull/12", "title": "feat",
            "state": "open", "draft": true, "head": {"ref": "ticket/dem-1"}, "base": {"ref": "main"},
        }),
    );
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/requested_reviewers",
        201,
        json!({}),
    );
    server.on("POST", "/repos/acme/demo/issues/12/labels", 200, json!([]));
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core
        .edit_ticket(&t.id, |x| {
            x.external = Some(ExternalRef {
                service: Service::GITHUB,
                id: "5".into(),
                key: "#5".into(),
                container: "acme/demo".into(),
                ..Default::default()
            });
            Ok(())
        })
        .unwrap();
    // The next one waits for it (« Après »).
    h.set_board(&p.id, |s| s.autopilot = true);
    let next = h
        .core
        .ticket_create(&p.id, crate::tickets_tests::draft("Suite [ok]", &[], 5))
        .await
        .unwrap();
    h.core
        .edit_ticket(&next.id, |x| {
            x.after = vec![t.id.clone()];
            Ok(())
        })
        .unwrap();
    h.core.ticket_approve(&t.id).await.unwrap();
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Pr, "{:?}", u.blocked);
    assert_eq!(u.outcome.as_deref(), Some("⇡ PR #12 → main"));
    assert_eq!(u.done_at, None);
    let tp = u.pr.clone().unwrap();
    assert_eq!(
        (tp.number, tp.word.as_str(), tp.kind.as_str()),
        (12, "PR", "github")
    );
    assert_eq!((tp.namespace.as_str(), tp.name.as_str()), ("acme", "demo"));
    let sent = server
        .writes()
        .into_iter()
        .find(|w| w.path() == "/repos/acme/demo/pulls")
        .expect("the pull request asked of GitHub")
        .json();
    assert_eq!(sent["draft"], true);
    assert_eq!(sent["head"], "ticket/dem-1");
    assert_eq!(sent["base"], "main");
    assert!(
        sent["body"].as_str().unwrap().ends_with("Closes #5"),
        "{}",
        sent["body"]
    );
    let asked: Vec<String> = server
        .writes()
        .iter()
        .map(|w| w.path().to_string())
        .collect();
    assert!(
        asked
            .iter()
            .any(|p| p == "/repos/acme/demo/pulls/12/requested_reviewers"),
        "{asked:?}"
    );
    assert!(
        asked
            .iter()
            .any(|p| p == "/repos/acme/demo/issues/12/labels"),
        "{asked:?}"
    );
    // In review, it holds the next one back.
    h.core.schedule_now().await;
    assert_eq!(h.ticket(&next.id).column, Column::Todo);

    // Still open: its card follows its CI, its approvals and its conflict.
    answer_pulls(&server, 12, "OPEN");
    assert_eq!(h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap(), 1);
    let tp = h.ticket(&t.id).pr.unwrap();
    assert_eq!(tp.checks, Some(CheckState::Success));
    assert_eq!((tp.approvals, tp.open_threads), (1, 1));
    assert_eq!(tp.mergeable, Some(false));
    assert_eq!(h.ticket(&t.id).column, Column::Pr);

    // The host out of reach: said on the card, the ticket stays.
    server.on(
        "POST",
        "/graphql#Pulls",
        502,
        json!({ "message": "Bad gateway" }),
    );
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Pr);
    assert!(
        u.pr.as_ref()
            .unwrap()
            .error
            .as_deref()
            .unwrap()
            .starts_with("GitHub"),
        "{:?}",
        u.pr
    );
    assert!(u.pr.as_ref().unwrap().error_since.is_some());

    // Merged on GitHub: done, its local branch gone (the board cleans up), the next one starts.
    answer_pulls(&server, 12, "MERGED");
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Done);
    assert_eq!(u.outcome.as_deref(), Some("⤵ PR #12 mergée dans main"));
    assert!(u.done_at.is_some());
    assert!(u.pr.unwrap().error.is_none());
    assert!(!has_branch(&r, "ticket/dem-1"));
    h.core.schedule_now().await;
    h.wait_ticket(&next.id, "started after the merge", |x| {
        x.column != Column::Todo
    })
    .await;
}

/// Closed without a merge: blocked, then « Terminer quand même » or « Remettre à faire ».
#[tokio::test]
async fn a_pull_request_closed_without_a_merge_blocks_the_ticket_with_two_ways_out() {
    let h = harness("tp-closed");
    let server = FakeServer::start().await;
    let (p, r) = board_on_github(&h, &server).await;
    h.set_board(&p.id, |s| {
        s.reviewers.clear();
        s.labels.clear();
    });
    let mut n = 20;
    for finish in [true, false] {
        n += 1;
        server.on(
            "POST",
            "/repos/acme/demo/pulls",
            201,
            json!({
                "number": n, "html_url": format!("https://github.com/acme/demo/pull/{n}"),
                "state": "open", "draft": false, "head": {"ref": "x"}, "base": {"ref": "main"},
            }),
        );
        h.set_board(&p.id, |s| s.autopilot = true);
        let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
        // Back to do, it would start again at once.
        h.set_board(&p.id, |s| s.autopilot = false);
        h.core.ticket_approve(&t.id).await.unwrap();
        assert_eq!(h.ticket(&t.id).column, Column::Pr);
        let branch = h.ticket(&t.id).pr.unwrap().branch;
        // Neither way out before it is closed.
        assert!(h.core.ticket_finish_anyway(&t.id).is_err());
        answer_pulls(&server, n, "CLOSED");
        h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
        let u = h.ticket(&t.id);
        assert_eq!(u.column, Column::Pr);
        assert_eq!(u.blocked, Some(format!("PR #{n} fermée sans merge")));
        if finish {
            h.core.ticket_finish_anyway(&t.id).unwrap();
            let u = h.ticket(&t.id);
            assert_eq!(u.column, Column::Done);
            assert_eq!(u.blocked, None);
            assert_eq!(
                u.outcome,
                Some(format!("✓ Terminé quand même · PR #{n} fermée"))
            );
        } else {
            h.core.ticket_pr_back_to_todo(&t.id).await.unwrap();
            let u = h.ticket(&t.id);
            assert_eq!(u.column, Column::Todo);
            assert_eq!((u.blocked, u.pr, u.agent_id), (None, None, None));
            // Its work stays: it takes its branch up again.
            assert!(has_branch(&r, &branch));
            assert_eq!(u.branch, branch);
        }
    }
    let _ = git;
}

/// Without an account of its host, the host's page opens, filled in; the ticket is done as before.
#[tokio::test]
async fn without_an_account_the_pull_request_is_finished_on_the_hosts_page() {
    let h = harness("tp-page");
    let (p, r) = h.project(false).await;
    crate::tickets_tests::github_remote(&h, &r);
    h.set_board(&p.id, |s| s.action = "pr".into());
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(t.column, Column::Done, "{:?}", t.blocked);
    assert_eq!(t.pr, None);
    assert_eq!(
        t.outcome.as_deref(),
        Some("⇡ ticket/dem-1 poussée · PR à finaliser")
    );
    assert!(t
        .outcome_url
        .unwrap()
        .starts_with("https://github.com/acme/demo/compare/main...ticket/dem-1?expand=1&title="));
}

/// GitLab's merge request (ESC-59, choice 3: « MR » on GitLab): « Draft: » before its title, its
/// labels, its reviewers by their ids, the issue it closes; merged there, the ticket is done.
#[tokio::test]
async fn on_gitlab_the_ticket_opens_a_merge_request_and_follows_it_to_its_merge() {
    let h = harness("tp-gitlab");
    let server = FakeServer::start().await;
    let api = "/api/v4";
    let project = format!("{api}/projects/acme%2Fdemo");
    server.on(
        "GET",
        &format!("{api}/user"),
        200,
        json!({ "username": "ada" }),
    );
    server.on(
        "GET",
        &format!("{api}/personal_access_tokens/self"),
        200,
        json!({ "scopes": ["api"] }),
    );
    h.core
        .forge_connect(crate::forge::connect::ForgeForm {
            kind: "gitlab".into(),
            url: server.url.clone(),
            token: "glpat-secret".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let (p, r) = h.project(false).await;
    let bare = h.dir.join("gitlab.git");
    git(
        &h.dir,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            &bare.to_string_lossy(),
        ],
    );
    let url = format!("{}/acme/demo.git", server.url);
    git(&r, &["remote", "add", "origin", &url]);
    let rule = format!(
        "url.{}.insteadOf",
        bare.to_string_lossy().replace('\\', "/")
    );
    git(&r, &["config", &rule, &url]);
    git(&r, &["push", "-qu", "origin", "main"]);
    h.set_board(&p.id, |s| {
        s.action = "pr".into();
        s.draft = true;
        s.reviewers = vec!["grace".into()];
        s.labels = vec!["escouade".into()];
    });
    server.on(
        "GET",
        &format!("{api}/users"),
        200,
        json!([{ "id": 7, "username": "grace" }]),
    );
    let mut mr: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/forges/gitlab/merge-request.json"
    ))
    .unwrap();
    mr["iid"] = json!(4);
    mr["web_url"] = json!(format!("{}/acme/demo/-/merge_requests/4", server.url));
    mr["state"] = json!("opened");
    mr["draft"] = json!(true);
    mr["merged_at"] = Value::Null;
    mr["source_branch"] = json!("ticket/dem-1");
    mr["target_branch"] = json!("main");
    mr["source_project_id"] = mr["target_project_id"].clone();
    server.on(
        "POST",
        &format!("{project}/merge_requests"),
        201,
        mr.clone(),
    );
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core
        .edit_ticket(&t.id, |x| {
            x.external = Some(ExternalRef {
                service: Service::GITLAB,
                id: "3".into(),
                key: "#3".into(),
                container: "acme/demo".into(),
                ..Default::default()
            });
            Ok(())
        })
        .unwrap();
    h.core.ticket_approve(&t.id).await.unwrap();
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Pr, "{:?}", u.blocked);
    assert_eq!(u.outcome.as_deref(), Some("⇡ MR #4 → main"));
    assert_eq!(u.pr.as_ref().unwrap().word, "MR");
    let sent = server
        .writes()
        .into_iter()
        .find(|w| w.path() == format!("{project}/merge_requests"))
        .expect("the merge request asked of GitLab")
        .json();
    assert!(
        sent["title"].as_str().unwrap().starts_with("Draft: "),
        "{}",
        sent["title"]
    );
    assert_eq!(sent["labels"], "escouade");
    assert_eq!(sent["reviewer_ids"], json!([7]));
    assert_eq!(sent["source_branch"], "ticket/dem-1");
    assert_eq!(sent["target_branch"], "main");
    assert!(
        sent["description"].as_str().unwrap().ends_with("Closes #3"),
        "{}",
        sent["description"]
    );

    // Merged on GitLab: done.
    mr["state"] = json!("merged");
    server.on("GET", &format!("{project}/merge_requests/4"), 200, mr);
    server.on(
        "GET",
        &format!("{project}/merge_requests/4/approvals"),
        200,
        json!({ "approved_by": [] }),
    );
    server.on(
        "GET",
        &format!("{project}/merge_requests/4/discussions"),
        200,
        json!([]),
    );
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Done, "{:?}", u.pr);
    assert_eq!(u.outcome.as_deref(), Some("⤵ MR #4 mergée dans main"));
}
