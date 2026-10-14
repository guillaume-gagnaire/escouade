//! Integration tests of the pull requests linked to an agent: real git repositories, an agent of
//! the fake `claude` CLI that opens one, GitHub told by the fake `gh` (tests/fixtures/fake-gh.mjs,
//! which answers from what a test wrote for the folder it runs in).

use crate::core_tests::{git, harness, Harness};
use crate::model::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn fake_gh() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(if cfg!(windows) {
            "fake-gh.cmd"
        } else {
            "fake-gh"
        })
}

/// The name of the fake `gh`'s files for the folder `cwd` it ran in.
fn gh_file(cwd: &Path, ext: &str) -> PathBuf {
    let key: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    std::env::temp_dir().join(format!("fake-gh-{key}.{ext}"))
}

/// What the fake `gh` was asked in `cwd`, each call's arguments joined (the fields left out).
fn asked(cwd: &Path) -> Vec<String> {
    std::fs::read_to_string(gh_file(cwd, "jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|l| {
            let call: Value = serde_json::from_str(l).unwrap();
            let argv: Vec<&str> = call["argv"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap())
                .collect();
            let end = argv
                .iter()
                .position(|a| *a == "--json")
                .unwrap_or(argv.len());
            argv[..end].join(" ")
        })
        .collect()
}

/// What GitHub has, for the fake `gh` run in `cwd`.
fn github(cwd: &Path, pulls: Value) {
    std::fs::write(gh_file(cwd, "pulls.json"), pulls.to_string()).unwrap();
}

/// A pull request as `gh` tells it, opened now.
fn pull(number: u32, head: &str, state: &str) -> Value {
    json!({
        "number": number, "url": format!("https://github.com/acme/demo/pull/{number}"),
        "title": format!("PR {number}"), "state": state, "isDraft": false,
        "createdAt": chrono::Utc::now().to_rfc3339(), "reviewDecision": "",
        "statusCheckRollup": [], "headRefName": head,
    })
}

/// The agent's pull requests, as (number, state, checks, review).
fn pulls_of(h: &Harness, id: &str) -> Vec<(u32, String, String, String)> {
    h.agent(id)
        .pull_requests
        .into_iter()
        .map(|p| (p.number, p.state, p.checks, p.review))
        .collect()
}

fn told(number: u32, state: &str, checks: &str, review: &str) -> (u32, String, String, String) {
    (number, state.into(), checks.into(), review.into())
}

/// Gives the repository `r` a bare remote, origin, and pushes `branch` from `from` to it.
fn push(h: &Harness, r: &Path, from: &Path, branch: &str) {
    let bare = h.dir.join("remote.git").to_string_lossy().to_string();
    git(&h.dir, &["init", "-q", "--bare", "-b", "main", &bare]);
    git(r, &["remote", "add", "origin", &bare]);
    git(from, &["push", "-qu", "origin", branch]);
}

#[tokio::test]
async fn the_pull_request_an_agent_opens_in_its_conversation_is_linked_to_it_and_read_on_github() {
    let h = harness("pulls-opened");
    let (p, r) = h.project(false).await;
    *h.core.gh_on_path.write() = Some(fake_gh());
    let mut opened = pull(12, "feat", "OPEN");
    opened["isDraft"] = json!(true);
    opened["statusCheckRollup"] = json!([{ "status": "IN_PROGRESS", "conclusion": "" }]);
    github(&r, json!([opened]));
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Ouvre la [pull-request]").await;
    // Linked as soon as `gh pr create` printed its address, then told by GitHub.
    h.wait("the pull request's state", |h| {
        pulls_of(h, &a) == [told(12, "draft", "pending", "")]
    })
    .await;
    let pr = h.agent(&a).pull_requests[0].clone();
    assert_eq!(
        (pr.url.as_str(), pr.title.as_str()),
        ("https://github.com/acme/demo/pull/12", "PR 12")
    );
    assert!(pr.checked_at > 0);
    // Without a branch of its own, only its address is asked.
    assert_eq!(asked(&r), ["pr view https://github.com/acme/demo/pull/12"]);
    // The window is sent the agent with it.
    let sent = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "agent" && e["agent"]["id"] == a.as_str())
        .cloned()
        .unwrap();
    assert_eq!(sent["agent"]["pullRequests"][0]["state"], "draft");
    assert_eq!(sent["agent"]["pullRequests"][0]["checks"], "pending");
}

#[tokio::test]
async fn an_agents_pull_requests_are_read_again_when_asked_and_no_more_once_merged() {
    let h = harness("pulls-refresh");
    let (p, r) = h.project(false).await;
    *h.core.gh_on_path.write() = Some(fake_gh());
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let url = "https://github.com/acme/demo/pull/7";
    assert!(h
        .core
        .agent(&a)
        .unwrap()
        .lock()
        .link_pull_request(7, url.into()));
    // GitHub does not know it (not signed in, another account's): it stays as it was.
    h.core.refresh_pull_requests(&a, false).await;
    assert_eq!(pulls_of(&h, &a), [told(7, "", "", "")]);
    let mut pr = pull(7, "feat", "OPEN");
    pr["reviewDecision"] = json!("CHANGES_REQUESTED");
    pr["statusCheckRollup"] = json!([{ "status": "COMPLETED", "conclusion": "FAILURE" }]);
    github(&r, json!([pr.clone()]));
    // Asked again at once, as a card clicked twice: not read again so soon…
    h.core.refresh_pull_requests(&a, false).await;
    assert_eq!(pulls_of(&h, &a), [told(7, "", "", "")]);
    // …unless something new calls for it.
    h.core.refresh_pull_requests(&a, true).await;
    assert_eq!(pulls_of(&h, &a), [told(7, "open", "failing", "changes")]);
    pr["state"] = json!("MERGED");
    github(&r, json!([pr]));
    h.core.refresh_pull_requests(&a, true).await;
    assert_eq!(pulls_of(&h, &a), [told(7, "merged", "", "")]);
    let calls = asked(&r).len();
    assert_eq!(calls, 3);
    // Merged: it will not change any more, GitHub is not asked.
    h.core.refresh_pull_requests(&a, true).await;
    h.core.refresh_all_pull_requests().await;
    assert_eq!(asked(&r).len(), calls);
    assert_eq!(pulls_of(&h, &a), [told(7, "merged", "", "")]);
}

#[tokio::test]
async fn the_pull_requests_of_an_agents_own_branch_are_found_once_it_is_pushed() {
    let h = harness("pulls-branch");
    let (p, r) = h.project(true).await;
    *h.core.gh_on_path.write() = Some(fake_gh());
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let wt = a.worktree.clone().expect("worktree created");
    let dir = PathBuf::from(&wt.path);
    let mut old = pull(3, &wt.branch, "MERGED");
    old["createdAt"] = json!("2020-01-01T00:00:00Z");
    let mut approved = pull(12, &wt.branch, "OPEN");
    approved["reviewDecision"] = json!("APPROVED");
    approved["statusCheckRollup"] = json!([{ "status": "COMPLETED", "conclusion": "SUCCESS" }]);
    github(
        &dir,
        json!([
            pull(14, &wt.branch, "CLOSED"),
            approved,
            old,
            pull(15, "another", "OPEN")
        ]),
    );
    // No remote has its branch yet: it has no pull request, GitHub is not asked.
    h.core.refresh_pull_requests(&a.id, true).await;
    assert_eq!(pulls_of(&h, &a.id), []);
    assert_eq!(asked(&dir), Vec::<String>::new());
    push(&h, &r, &dir, &wt.branch);
    // Whoever opened them: those of its branch since the agent is there, the oldest first. Not
    // the one an earlier branch of that name had, nor another branch's.
    h.core.refresh_pull_requests(&a.id, true).await;
    assert_eq!(
        pulls_of(&h, &a.id),
        [
            told(12, "open", "passing", "approved"),
            told(14, "closed", "", "")
        ]
    );
    assert_eq!(
        asked(&dir),
        [format!(
            "pr list --head {} --state all --limit 20",
            wt.branch
        )]
    );
    // At startup: every agent that may have some is asked.
    let mut merged = pull(12, &wt.branch, "MERGED");
    merged["reviewDecision"] = json!("APPROVED");
    github(&dir, json!([merged, pull(14, &wt.branch, "CLOSED")]));
    h.core.pr_reads.lock().clear();
    h.core.refresh_all_pull_requests().await;
    assert_eq!(
        pulls_of(&h, &a.id),
        [told(12, "merged", "", ""), told(14, "closed", "", "")]
    );
}

#[tokio::test]
async fn a_pull_request_of_someone_elses_fork_whose_branch_bears_the_same_name_is_not_the_agents() {
    let h = harness("pulls-owner");
    let (p, r) = h.project(true).await;
    *h.core.gh_on_path.write() = Some(fake_gh());
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let wt = a.worktree.clone().expect("worktree created");
    let dir = PathBuf::from(&wt.path);
    push(&h, &r, &dir, &wt.branch);
    // Pushed, its remote is now said to be Ada's repository on GitHub.
    git(
        &r,
        &["remote", "set-url", "origin", "git@github.com:Ada/demo.git"],
    );
    let of = |number: u32, owner: Value| {
        let mut pr = pull(number, &wt.branch, "OPEN");
        pr["headRepositoryOwner"] = owner;
        pr
    };
    github(
        &dir,
        json!([
            of(20, json!({ "login": "ada" })),
            of(21, json!({ "login": "mallory" })),
            // Its fork deleted since: nobody to tell it from.
            of(22, Value::Null),
        ]),
    );
    h.core.refresh_pull_requests(&a.id, true).await;
    assert_eq!(
        pulls_of(&h, &a.id),
        [told(20, "open", "", ""), told(22, "open", "", "")]
    );
}

#[tokio::test]
async fn a_pull_request_whose_repository_was_renamed_stays_one_link_under_its_address() {
    let h = harness("pulls-renamed");
    let (p, r) = h.project(false).await;
    *h.core.gh_on_path.write() = Some(fake_gh());
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let old = "https://github.com/acme/old/pull/12";
    assert!(h
        .core
        .agent(&a)
        .unwrap()
        .lock()
        .link_pull_request(12, old.into()));
    // Asked at the address it was linked by, GitHub answers with the new one.
    let mut pr = pull(12, "feat", "MERGED");
    pr["was"] = json!(old);
    pr["url"] = json!("https://github.com/acme/new/pull/12");
    github(&r, json!([pr]));
    h.core.refresh_pull_requests(&a, true).await;
    let linked = h.agent(&a).pull_requests;
    assert_eq!(linked.len(), 1, "{linked:?}");
    assert_eq!(
        (linked[0].url.as_str(), linked[0].state.as_str()),
        (old, "merged")
    );
}

#[tokio::test]
async fn a_token_that_may_not_read_the_checks_still_tells_the_state_and_the_reviews() {
    let h = harness("pulls-no-checks");
    let (p, r) = h.project(false).await;
    *h.core.gh_on_path.write() = Some(fake_gh());
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let url = "https://github.com/acme/demo/pull/7";
    assert!(h
        .core
        .agent(&a)
        .unwrap()
        .lock()
        .link_pull_request(7, url.into()));
    let mut pr = pull(7, "feat", "OPEN");
    pr["reviewDecision"] = json!("APPROVED");
    pr["statusCheckRollup"] = json!([{ "status": "COMPLETED", "conclusion": "FAILURE" }]);
    github(&r, json!({ "forbidChecks": true, "pulls": [pr] }));
    h.core.refresh_pull_requests(&a, true).await;
    // Asked with its checks, refused, then without them.
    assert_eq!(pulls_of(&h, &a), [told(7, "open", "", "approved")]);
    assert_eq!(
        asked(&r),
        [format!("pr view {url}"), format!("pr view {url}")]
    );
}

#[tokio::test]
async fn nothing_is_kept_of_the_reading_of_an_agent_that_is_gone() {
    let h = harness("pulls-gone");
    let (p, _r) = h.project(false).await;
    *h.core.gh_on_path.write() = Some(fake_gh());
    h.core.refresh_pull_requests("gone", true).await;
    assert!(h.core.pr_reads.lock().is_empty());
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.refresh_pull_requests(&a, true).await;
    assert!(h.core.pr_reads.lock().contains_key(&a));
    h.core.delete_agent(&a, false).await.unwrap();
    assert!(h.core.pr_reads.lock().is_empty());
    // Asked once more after it went (a reading asked just before): still nothing.
    h.core.refresh_pull_requests(&a, true).await;
    assert!(h.core.pr_reads.lock().is_empty());
}

#[tokio::test]
async fn without_gh_a_pull_request_stays_linked_with_its_address_alone() {
    let h = harness("pulls-no-gh");
    let (p, _r) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Ouvre la [pull-request]").await;
    h.core.refresh_pull_requests(&a, true).await;
    h.core.refresh_all_pull_requests().await;
    let linked = h.agent(&a).pull_requests;
    assert_eq!(linked.len(), 1);
    assert_eq!(
        (
            linked[0].number,
            linked[0].url.as_str(),
            linked[0].state.as_str()
        ),
        (12, "https://github.com/acme/demo/pull/12", "")
    );
    // Kept with the agent: there again after a restart.
    let saved: Value = serde_json::to_value(h.agent(&a)).unwrap();
    assert_eq!(saved["pullRequests"][0]["number"], 12);
    let back: AgentMeta = serde_json::from_value(saved).unwrap();
    assert_eq!(back.pull_requests, linked);
}
