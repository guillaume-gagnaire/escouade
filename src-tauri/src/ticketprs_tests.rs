//! The pull requests of the tickets (ESC-59): what they open with, the page without an account,
//! their reading and what it does to the ticket, against a fake GitHub served by its plugin.

use super::*;
use crate::agentprs_tests::github_project;
use crate::core_tests::{git, harness, Harness};
use crate::i18n::Lang::{En, Fr};
use crate::integrations::fake::FakeServer;
use crate::model::{BoardSettings, ExternalRef, FailedCheck, Service, Ticket};
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
    // The next one waits for it (« Après »), set at its creation: the autopilot is on (a board's default), and an
    // « Après » set just after could come once it started the next one (the CI's Windows, 2026-11-02, twice).
    let mut after_it = crate::tickets_tests::draft("Suite [ok]", &[], 5);
    after_it.after = vec![t.id.clone()];
    let next = h.core.ticket_create(&p.id, after_it).await.unwrap();
    assert_eq!(h.ticket(&next.id).after, vec![t.id.clone()]);
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

// ---------- Merging from Escouade (ESC-62) ----------

fn ready(tp: &mut TicketPr) {
    tp.approvals = 1;
    tp.checks = Some(CheckState::Success);
    tp.mergeable = Some(true);
}

#[test]
fn merge_is_offered_only_approved_and_green_and_merge_when_green_while_the_ci_runs() {
    let mut t = in_review(12);
    let tp = t.pr.as_mut().unwrap();
    // Open, not approved: neither.
    tp.checks = Some(CheckState::Success);
    assert!(!merge_ready(tp));
    ready(tp);
    assert!(merge_ready(tp));
    // No CI at all is green too.
    tp.checks = None;
    assert!(merge_ready(tp));
    let spoils: [fn(&mut TicketPr); 8] = [
        |p| p.checks = Some(CheckState::Failure),
        |p| p.checks = Some(CheckState::Pending),
        |p| p.changes_requested = true,
        |p| p.mergeable = Some(false),
        |p| p.draft = true,
        |p| p.closed = true,
        |p| p.state = PrState::Merged,
        // Its agent fixes its CI (ESC-60): nothing merges before the fix is pushed.
        |p| p.fixing = Some(1),
    ];
    for spoil in spoils {
        let mut p = tp.clone();
        ready(&mut p);
        spoil(&mut p);
        assert!(!merge_ready(&p), "{p:?}");
    }
    // While its CI runs: « Merger quand c'est vert », once.
    let mut p = tp.clone();
    p.checks = Some(CheckState::Pending);
    assert!(can_wait_green(&p));
    p.merge_when_green = Some(MergeStrategy::Squash);
    assert!(!can_wait_green(&p));
    p.merge_when_green = None;
    p.auto_merge = true;
    assert!(!can_wait_green(&p));
    p.auto_merge = false;
    p.checks = Some(CheckState::Success);
    assert!(!can_wait_green(&p), "green already: « Merger »");
}

#[test]
fn the_kanbans_strategy_is_taken_when_the_repository_allows_it_else_one_it_allows() {
    use MergeStrategy::*;
    let opts = |s: &[MergeStrategy]| MergeOptions {
        strategies: s.to_vec(),
        auto_merge: false,
    };
    assert_eq!(strategy_of("squash"), Squash);
    assert_eq!(strategy_of("merge"), Merge);
    assert_eq!(strategy_of("rebase"), Rebase);
    let all = opts(&[Merge, Squash, Rebase]);
    assert_eq!(pick_strategy(Rebase, Some(&all)), Some(Rebase));
    assert_eq!(
        pick_strategy(Squash, Some(&opts(&[Merge, Rebase]))),
        Some(Merge)
    );
    assert_eq!(pick_strategy(Squash, Some(&opts(&[]))), None);
    assert_eq!(pick_strategy(Squash, None), None, "not read yet");
    let caps = Capabilities {
        auto_merge: true,
        merge_strategies: vec![Merge],
        ..Default::default()
    };
    assert_eq!(
        options_of_capabilities(&caps),
        MergeOptions {
            strategies: vec![Merge],
            auto_merge: true
        }
    );
}

#[test]
fn escouade_merges_what_it_waits_for_once_approved_and_green_and_stops_on_a_red_ci() {
    let mut t = in_review(12);
    t.pr.as_mut().unwrap().merge_when_green = Some(MergeStrategy::Merge);
    let mut p = pr(12, PrState::Open);
    p.checks = Some(CheckState::Pending);
    p.approvals = 1;
    assert_eq!(settle(&mut t, &p, None, 2, Fr), News::Same);
    p.checks = Some(CheckState::Success);
    assert_eq!(
        settle(&mut t, &p, None, 3, Fr),
        News::MergeNow(MergeStrategy::Merge)
    );
    // Green but not approved: it waits on.
    p.approvals = 0;
    assert_eq!(settle(&mut t, &p, None, 4, Fr), News::Same);
    assert!(t.pr.as_ref().unwrap().merge_when_green.is_some());
    // Red: the wait is over, the card says so.
    p.checks = Some(CheckState::Failure);
    assert_eq!(settle(&mut t, &p, None, 5, Fr), News::Same);
    let tp = t.pr.as_ref().unwrap();
    assert_eq!(tp.merge_when_green, None);
    assert!(tp.merge_note.as_deref().unwrap().starts_with("CI en échec"));
    // The host's auto-merge is read on the card.
    p.auto_merge = true;
    settle(&mut t, &p, None, 6, Fr);
    assert!(t.pr.as_ref().unwrap().auto_merge);
}

fn node_of(number: u64, state: &str, rollup: &str, approved: bool) -> Value {
    let reviews = if approved {
        json!([{"state": "APPROVED"}])
    } else {
        json!([])
    };
    json!({
        "number": number, "url": format!("https://github.com/acme/demo/pull/{number}"),
        "title": "feat", "state": state, "isDraft": false,
        "headRefName": "ticket/dem-1", "baseRefName": "main", "mergeable": "MERGEABLE",
        "isCrossRepository": false, "autoMergeRequest": null,
        "latestOpinionatedReviews": {"nodes": reviews},
        "reviewThreads": {"nodes": []},
        "commits": {"nodes": [{"commit": {"statusCheckRollup": {"state": rollup}}}]},
    })
}

fn pulls_answer(number: u64, state: &str, rollup: &str, approved: bool) -> Value {
    json!({ "data": { "repository": { format!("p{number}"): node_of(number, state, rollup, approved) } } })
}

/// A ticket « En revue » with its pull request #12 on the fake GitHub, whose repository has
/// `settings`.
async fn merge_board(h: &Harness, server: &FakeServer, settings: Value) -> (String, String) {
    let (p, _) = board_on_github(h, server).await;
    server.on("GET", "/repos/acme/demo", 200, settings);
    let (t, _) = reviewed(h, &p.id, "Fichier [ok]").await;
    h.core
        .edit_ticket(&t.id, |x| {
            x.column = Column::Pr;
            x.pr = in_review(12).pr;
            Ok(())
        })
        .unwrap();
    (p.id, t.id)
}

fn wrote(server: &FakeServer, path: &str) -> Option<Value> {
    server
        .writes()
        .into_iter()
        .find(|w| w.path() == path)
        .map(|w| w.json())
}

fn graphql_sent(server: &FakeServer, op: &str) -> Option<Value> {
    server
        .writes()
        .into_iter()
        .map(|w| w.json())
        .find(|j| j["operationName"] == op)
}

/// « Merger »: refused before it is approved and green; then merged with a strategy the
/// repository allows (squash is not: a merge commit), and the ticket is done.
#[tokio::test]
async fn merge_goes_approved_and_green_with_a_strategy_the_repository_allows() {
    let h = harness("tp-merge-now");
    let server = FakeServer::start().await;
    let settings = json!({
        "allow_merge_commit": true, "allow_squash_merge": false, "allow_rebase_merge": false,
        "allow_auto_merge": false,
    });
    let (p, id) = merge_board(&h, &server, settings).await;
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "OPEN", "SUCCESS", false),
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    let tp = h.ticket(&id).pr.unwrap();
    assert_eq!(
        tp.merge_options.as_ref().map(|o| o.strategies.clone()),
        Some(vec![MergeStrategy::Merge])
    );
    assert!(
        h.core.ticket_pr_merge(&id, false).await.is_err(),
        "not approved"
    );
    assert!(wrote(&server, "/repos/acme/demo/pulls/12/merge").is_none());

    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "MERGED", "SUCCESS", true),
    );
    server.on_once(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "OPEN", "SUCCESS", true),
        &[],
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    server.on(
        "PUT",
        "/repos/acme/demo/pulls/12/merge",
        200,
        json!({ "merged": true }),
    );
    assert_eq!(
        h.core.ticket_pr_merge(&id, false).await.unwrap(),
        MergeHow::Merged
    );
    let sent = wrote(&server, "/repos/acme/demo/pulls/12/merge").unwrap();
    assert_eq!(sent["merge_method"], "merge");
    let u = h.ticket(&id);
    assert_eq!(u.column, Column::Done);
    assert_eq!(u.outcome.as_deref(), Some("⤵ PR #12 mergée dans main"));
}

/// « Merger quand c'est vert » on a repository that allows auto-merge: GitHub's own.
#[tokio::test]
async fn merge_when_green_turns_on_the_hosts_auto_merge_when_the_repository_allows_it() {
    let h = harness("tp-auto-merge");
    let server = FakeServer::start().await;
    let settings = json!({
        "allow_merge_commit": true, "allow_squash_merge": true, "allow_rebase_merge": true,
        "allow_auto_merge": true,
    });
    let (p, id) = merge_board(&h, &server, settings).await;
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "OPEN", "PENDING", true),
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert!(
        h.core.ticket_pr_merge(&id, false).await.is_err(),
        "not green"
    );
    server.on(
        "POST",
        "/graphql#PullRequestId",
        200,
        json!({ "data": { "repository": { "pullRequest": { "id": "PR_12" } } } }),
    );
    server.on(
        "POST",
        "/graphql#EnableAutoMerge",
        200,
        json!({ "data": { "enablePullRequestAutoMerge": { "pullRequest": {
            "number": 12, "autoMergeRequest": { "enabledAt": "2026-11-02T10:00:00Z" } } } } }),
    );
    assert_eq!(
        h.core.ticket_pr_merge(&id, true).await.unwrap(),
        MergeHow::Host
    );
    let sent = graphql_sent(&server, "EnableAutoMerge").expect("auto-merge asked of GitHub");
    assert_eq!(sent["variables"]["method"], "SQUASH");
    let tp = h.ticket(&id).pr.unwrap();
    assert!(tp.auto_merge);
    assert_eq!(tp.merge_when_green, None);
    // Once on, it is not offered again.
    assert!(h.core.ticket_pr_merge(&id, true).await.is_err());
}

/// « Merger quand c'est vert » without auto-merge on the repository: Escouade waits, then merges
/// itself at the end of the CI.
#[tokio::test]
async fn without_the_hosts_auto_merge_escouade_merges_at_the_end_of_the_ci() {
    let h = harness("tp-wait-green");
    let server = FakeServer::start().await;
    let settings = json!({
        "allow_merge_commit": true, "allow_squash_merge": true, "allow_rebase_merge": true,
        "allow_auto_merge": false,
    });
    let (p, id) = merge_board(&h, &server, settings).await;
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "OPEN", "PENDING", true),
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert_eq!(
        h.core.ticket_pr_merge(&id, true).await.unwrap(),
        MergeHow::Escouade
    );
    assert_eq!(
        h.ticket(&id).pr.unwrap().merge_when_green,
        Some(MergeStrategy::Squash)
    );
    assert!(graphql_sent(&server, "EnableAutoMerge").is_none());
    // Still running: nothing merges.
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert!(wrote(&server, "/repos/acme/demo/pulls/12/merge").is_none());
    // Green: merged by Escouade, the ticket done.
    server.on(
        "PUT",
        "/repos/acme/demo/pulls/12/merge",
        200,
        json!({ "merged": true }),
    );
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "MERGED", "SUCCESS", true),
    );
    server.on_once(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "OPEN", "SUCCESS", true),
        &[],
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    let sent = wrote(&server, "/repos/acme/demo/pulls/12/merge").expect("merged by Escouade");
    assert_eq!(sent["merge_method"], "squash");
    assert_eq!(h.ticket(&id).column, Column::Done);
}

/// The host refuses the merge Escouade waited for: said on the card, the wait over.
#[tokio::test]
async fn a_merge_refused_by_the_host_is_said_on_the_card() {
    let h = harness("tp-merge-refused");
    let server = FakeServer::start().await;
    let (p, id) = merge_board(&h, &server, json!({ "allow_auto_merge": false })).await;
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "OPEN", "PENDING", true),
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    h.core.ticket_pr_merge(&id, true).await.unwrap();
    server.on(
        "PUT",
        "/repos/acme/demo/pulls/12/merge",
        405,
        json!({ "message": "Required status check \"ci\" is expected." }),
    );
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        pulls_answer(12, "OPEN", "SUCCESS", true),
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    let u = h.ticket(&id);
    assert_eq!(u.column, Column::Pr);
    let tp = u.pr.unwrap();
    assert_eq!(tp.merge_when_green, None);
    assert!(
        tp.merge_note
            .as_deref()
            .unwrap()
            .starts_with("Merge refusé"),
        "{:?}",
        tp.merge_note
    );
    // « Ne plus attendre » clears it.
    h.core.ticket_pr_stop_waiting(&id).unwrap();
    assert_eq!(h.ticket(&id).pr.unwrap().merge_note, None);
}

// ---------- Its CI fixed by its agent (ESC-60) ----------

fn failing(t: &mut Ticket) {
    let tp = t.pr.as_mut().unwrap();
    tp.checks = Some(CheckState::Failure);
}

fn on_board(id: &str, column: Column) -> Ticket {
    Ticket {
        id: id.into(),
        project_id: "p".into(),
        column,
        max_loops: 3,
        agent_id: Some(format!("a-{id}")),
        ..Default::default()
    }
}

#[test]
fn a_reading_keeps_the_failed_checks_and_which_ones_have_a_log() {
    let mut t = in_review(12);
    let mut p = pr(12, PrState::Open);
    p.checks = Some(CheckState::Failure);
    let list = vec![
        Check {
            name: "test".into(),
            state: CheckState::Failure,
            job: Some("55".into()),
            url: Some("https://github.com/acme/demo/runs/55".into()),
            ..Default::default()
        },
        Check {
            name: "ci/jenkins".into(),
            state: CheckState::Failure,
            url: Some("https://jenkins.example.com/1".into()),
            ..Default::default()
        },
        check(CheckState::Success),
    ];
    t.pr.as_mut().unwrap().awaiting_ci = true;
    settle(&mut t, &p, Some(&list), 2, Fr);
    let tp = t.pr.clone().unwrap();
    assert_eq!(
        tp.failed,
        vec![
            FailedCheck {
                name: "test".into(),
                url: Some("https://github.com/acme/demo/runs/55".into()),
                log: true
            },
            FailedCheck {
                name: "ci/jenkins".into(),
                url: Some("https://jenkins.example.com/1".into()),
                log: false
            },
        ]
    );
    // Still failing: what was fixed is not given again by the autopilot.
    assert!(tp.awaiting_ci);
    // Run again since: the next failure goes to its agent.
    p.checks = Some(CheckState::Pending);
    t.pr.as_mut().unwrap().fix_error = Some("Push impossible".into());
    settle(&mut t, &p, Some(&[check(CheckState::Pending)]), 3, Fr);
    let tp = t.pr.clone().unwrap();
    assert!(!tp.awaiting_ci && tp.failed.is_empty());
    assert_eq!(tp.fix_error.as_deref(), Some("Push impossible"));
    // Green: the error of the last fix goes.
    p.checks = Some(CheckState::Success);
    settle(&mut t, &p, Some(&[check(CheckState::Success)]), 4, Fr);
    assert_eq!(t.pr.unwrap().fix_error, None);
}

#[test]
fn the_autopilot_gives_a_failed_ci_to_its_agent_within_the_loops_and_the_places() {
    let s = |autopilot: bool, max_parallel: u32| BoardSettings {
        autopilot,
        max_parallel,
        ..Default::default()
    };
    let mut a = in_review(1);
    a.project_id = "p".into();
    a.max_loops = 3;
    a.agent_id = Some("a1".into());
    let green = a.clone();
    failing(&mut a);
    assert!(fixable(&a) && auto_fixable(&a));
    assert!(!fixable(&green));
    let tickets = vec![a.clone(), green];
    assert_eq!(to_fix(&tickets, "p", &s(true, 2)), vec!["t1".to_string()]);
    // Without the autopilot, only « L'agent corrige ».
    assert!(to_fix(&tickets, "p", &s(false, 2)).is_empty());
    assert!(to_fix(&tickets, "other", &s(true, 2)).is_empty());
    // No place left: a ticket « En cours » holds it.
    let doing = on_board("d", Column::Doing);
    assert!(to_fix(&[a.clone(), doing.clone()], "p", &s(true, 1)).is_empty());
    assert_eq!(to_fix(&[a.clone(), doing], "p", &s(true, 2)).len(), 1);

    // Begun: counted as a loop, it holds a place (the next ticket to do waits), and is not given
    // twice.
    let mut b = a.clone();
    fix_begins(&mut b, Some("abc".into()), 5);
    let tp = b.pr.clone().unwrap();
    assert_eq!((tp.fixing, tp.fixes, b.loops), (Some(5), 1, 1));
    assert!(fixing(&b) && !fixable(&b));
    assert_eq!(busy(std::slice::from_ref(&b), "p"), 1);
    let todo = Ticket {
        id: "n".into(),
        project_id: "p".into(),
        column: Column::Todo,
        ..Default::default()
    };
    assert!(board::to_start(&[b.clone(), todo.clone()], "p", &s(true, 1), |_| false).is_empty());
    assert_eq!(
        board::to_start(&[a.clone(), todo], "p", &s(true, 1), |_| false),
        vec!["n".to_string()]
    );

    // Fixed and pushed: back « En revue », its CI to run again before the autopilot gives it again.
    fix_done(&mut b);
    assert!(!fixing(&b) && fixable(&b) && !auto_fixable(&b));
    // Failed: said, given again by hand only.
    let mut c = a.clone();
    fix_begins(&mut c, None, 5);
    fix_failed(&mut c, "Interrompu".into());
    let tp = c.pr.clone().unwrap();
    assert_eq!(tp.fix_error.as_deref(), Some("Interrompu"));
    assert!(fixable(&c) && !auto_fixable(&c));

    // Its loops spent: by hand only.
    let mut d = a.clone();
    d.pr.as_mut().unwrap().fixes = 3;
    assert!(fixable(&d) && !auto_fixable(&d));
    // Without an agent, or closed: none.
    let mut e = a.clone();
    e.agent_id = None;
    assert!(!fixable(&e));
    let mut f = a.clone();
    f.pr.as_mut().unwrap().closed = true;
    assert!(!fixable(&f));
}

#[test]
fn the_end_of_the_fixing_turn_pushes_waits_or_says_why() {
    use crate::board::TurnEnd;
    assert_eq!(fix_end(&TurnEnd::Finished("ok".into())), FixEnd::Finished);
    assert_eq!(fix_end(&TurnEnd::Limited), FixEnd::Wait);
    assert_eq!(fix_end(&TurnEnd::SignIn), FixEnd::Wait);
    assert!(matches!(fix_end(&TurnEnd::Interrupted), FixEnd::Stopped(_)));
    assert!(matches!(
        fix_end(&TurnEnd::Error("boom\nmore".into())),
        FixEnd::Stopped(w) if w.contains("boom") && !w.contains("more")
    ));
    assert_eq!(
        nothing_fixed(En),
        "The agent committed nothing: the CI still fails"
    );
    assert!(not_committed(Fr).contains("pas commités"));
    assert_eq!(not_pushed(Fr, "rejeté"), "Push impossible : rejeté");
}

/// The CI of PR #12 on the fake GitHub: `rollup` for its state; failing, a GitHub Actions job
/// (55) with its log and a status of another CI (Jenkins) with none.
fn answer_ci(server: &FakeServer, rollup: &str) {
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        json!({ "data": { "repository": { "p12": graphql_pr(12, "OPEN", rollup) } } }),
    );
    server.on(
        "GET",
        "/repos/acme/demo/pulls/12",
        200,
        json!({ "number": 12, "head": { "sha": "abc" } }),
    );
    let failed = rollup == "FAILURE";
    server.on(
        "GET",
        "/repos/acme/demo/commits/abc/check-runs",
        200,
        json!({ "check_runs": [
            { "name": "test", "status": "completed",
              "conclusion": if failed { "failure" } else { "success" }, "id": 55,
              "html_url": "https://github.com/acme/demo/runs/55", "app": { "slug": "github-actions" } },
        ] }),
    );
    let statuses = if failed {
        json!([{ "context": "ci/jenkins", "state": "failure", "target_url": "https://jenkins.example.com/job/7" }])
    } else {
        json!([])
    };
    server.on(
        "GET",
        "/repos/acme/demo/commits/abc/status",
        200,
        json!({ "statuses": statuses }),
    );
    server.on_text(
        "GET",
        "/repos/acme/demo/actions/jobs/55/logs",
        200,
        "npm test\nFAIL src/login.test.ts\n",
        &[],
    );
}

fn told(h: &Harness, id: &str, what: &str) -> bool {
    h.items(id)
        .iter()
        .any(|i| i["kind"] == "user" && i["text"].as_str().is_some_and(|t| t.contains(what)))
}

fn on_origin(bare: &std::path::Path, branch: &str) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(bare)
        .args(["log", "--format=%s", branch])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn pr_opens(server: &FakeServer) {
    server.on(
        "POST",
        "/repos/acme/demo/pulls",
        201,
        json!({
            "number": 12, "html_url": "https://github.com/acme/demo/pull/12", "title": "feat",
            "state": "open", "draft": false, "head": {"ref": "ticket/dem-1"}, "base": {"ref": "main"},
        }),
    );
}

/// A ticket « En revue » whose CI failed, with the autopilot: its agent, archived at the
/// validation, is restored with its worktree made again from its branch (gone from the project:
/// fetched from its remote), gets the failed jobs and the end of their logs, commits; Escouade
/// pushes, and the ticket is back « En revue », its agent archived again. The same failure is not
/// given twice; by hand, « L'agent corrige » gives it again.
#[tokio::test]
async fn with_the_autopilot_a_failed_ci_goes_back_to_its_agent_and_the_ticket_back_in_review() {
    let h = harness("tp-fix");
    let server = FakeServer::start().await;
    let (p, r, bare) = github_project(&h, &server, false).await;
    h.set_board(&p.id, |s| s.action = "pr".into());
    pr_opens(&server);
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Pr, "{:?}", u.blocked);
    let agent_id = u.agent_id.clone().unwrap();
    assert!(h.agent(&agent_id).archived);
    assert!(!wt.exists(), "its worktree goes at the validation");
    // Its branch deleted from the project by hand: it is on the remote still.
    git(&r, &["branch", "-D", "ticket/dem-1"]);
    h.set_board(&p.id, |s| s.autopilot = false);

    // Its CI fails: without the autopilot, nothing goes by itself.
    answer_ci(&server, "FAILURE");
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    h.core.schedule_now().await;
    let tp = h.ticket(&t.id).pr.unwrap();
    assert_eq!(tp.checks, Some(CheckState::Failure));
    assert_eq!((tp.fixing, tp.fixes), (None, 0));
    assert_eq!(
        tp.failed
            .iter()
            .map(|c| (c.name.as_str(), c.log))
            .collect::<Vec<_>>(),
        vec![("test", true), ("ci/jenkins", false)]
    );

    // With it, its agent fixes it.
    let loops = h.ticket(&t.id).loops;
    h.set_board(&p.id, |s| s.autopilot = true);
    h.core.schedule_now().await;
    h.wait("the failed jobs sent", |h| {
        told(h, &agent_id, "La CI de la PR #12")
    })
    .await;
    assert!(told(
        &h,
        &agent_id,
        "<check name=\"test\" url=\"https://github.com/acme/demo/runs/55\"/>"
    ));
    assert!(told(&h, &agent_id, "<check name=\"ci/jenkins\""));
    assert!(told(&h, &agent_id, "FAIL src/login.test.ts"));
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Pr);
    assert_eq!(u.loops, loops + 1);
    assert_eq!(u.pr.as_ref().unwrap().fixes, 1);
    assert!(has_branch(&r, "ticket/dem-1"), "its branch fetched again");

    // Its turn over: pushed, back « En revue », its agent archived again, its worktree gone.
    h.wait_ticket(&t.id, "the fix over", |x| {
        x.pr.as_ref().is_some_and(|p| p.fixing.is_none())
    })
    .await;
    let u = h.ticket(&t.id);
    let tp = u.pr.clone().unwrap();
    assert_eq!(u.column, Column::Pr);
    assert_eq!(tp.fix_error, None, "{:?}", tp.fix_error);
    assert!(tp.awaiting_ci);
    assert!(
        on_origin(&bare, "ticket/dem-1").contains("fix: CI de DEM-1"),
        "{}",
        on_origin(&bare, "ticket/dem-1")
    );
    h.wait("its agent archived again", |h| h.agent(&agent_id).archived)
        .await;
    let restored = h.agent(&agent_id).worktree.unwrap().path;
    h.wait("its worktree removed again", |_| {
        !std::path::Path::new(&restored).exists()
    })
    .await;

    // The same failure read again: not given twice.
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    h.core.schedule_now().await;
    assert_eq!(h.ticket(&t.id).pr.unwrap().fixes, 1);
    // By hand, it is.
    h.set_board(&p.id, |s| s.autopilot = false);
    h.core.ticket_pr_fix_ci(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).pr.unwrap().fixes, 2);
    // A second click meanwhile is refused.
    assert!(h.core.ticket_pr_fix_ci(&t.id).await.is_err());
    h.wait_ticket(&t.id, "the second fix over", |x| {
        x.pr.as_ref().is_some_and(|p| p.fixing.is_none())
    })
    .await;
    assert_eq!(h.ticket(&t.id).pr.unwrap().fix_error, None);
    assert_eq!(
        on_origin(&bare, "ticket/dem-1")
            .matches("fix: CI de DEM-1")
            .count(),
        2
    );

    // Green: nothing to fix, refused.
    answer_ci(&server, "SUCCESS");
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    let u = h.ticket(&t.id);
    assert!(u.pr.as_ref().unwrap().failed.is_empty());
    assert!(h.core.ticket_pr_fix_ci(&t.id).await.is_err());
}

/// The agent commits nothing: the card says so, the autopilot does not give it again, and its
/// agent stays restored to be talked to.
#[tokio::test]
async fn an_agent_that_commits_nothing_is_said_on_the_card() {
    let h = harness("tp-fix-none");
    let server = FakeServer::start().await;
    let (p, _r, _) = github_project(&h, &server, false).await;
    h.set_board(&p.id, |s| s.action = "pr".into());
    pr_opens(&server);
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok] [ci-rien]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    answer_ci(&server, "FAILURE");
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    h.core.ticket_pr_fix_ci(&t.id).await.unwrap();
    h.wait_ticket(&t.id, "the fix over", |x| {
        x.pr.as_ref().is_some_and(|p| p.fixing.is_none())
    })
    .await;
    let u = h.ticket(&t.id);
    let tp = u.pr.clone().unwrap();
    assert_eq!(tp.fix_error.as_deref(), Some(nothing_fixed(Fr).as_str()));
    assert!(!auto_fixable(&u));
    let agent_id = u.agent_id.unwrap();
    assert!(!h.agent(&agent_id).archived);
}
