// « Lancer un agent sur cette PR » (ESC-65): how a pull request's branch is fetched, its own or a
// fork's on each host; and, on real repositories, the worktree an agent gets on it.

use crate::forge::{PullFetch, PullRequest};
use crate::pragent::{fetch_plan, fork_branch, FetchPlan};

fn pr(number: u64, head: &str, fork: bool, head_repo: &str) -> PullRequest {
    PullRequest {
        number,
        head: head.into(),
        base: "main".into(),
        fork,
        head_repo: head_repo.into(),
        ..Default::default()
    }
}

#[test]
fn its_own_branch_comes_through_the_remote_tracking_one_or_the_local_one() {
    let p = pr(41, "feat/login", false, "");
    assert_eq!(
        fetch_plan(&p, "origin", "https://github.com", None, "PR", false).unwrap(),
        FetchPlan {
            from: "origin".into(),
            refspec: "+refs/heads/feat/login:refs/remotes/origin/feat/login".into(),
            branch: "origin/feat/login".into(),
            second_parent: false,
        }
    );
    // A local branch of that name: the agent works on it, as « Nouvel agent sur une branche… ».
    let plan = fetch_plan(&p, "upstream", "https://github.com", None, "PR", true).unwrap();
    assert_eq!(plan.branch, "feat/login");
    assert_eq!(plan.from, "upstream");
}

#[test]
fn a_forks_pull_request_comes_by_the_reference_its_host_keeps() {
    let p = pr(127, "i18n-es", true, "marta/demo");
    let gh = Some(PullFetch::Head("refs/pull/127/head".into()));
    assert_eq!(
        fetch_plan(&p, "origin", "https://github.com", gh, "PR", false).unwrap(),
        FetchPlan {
            from: "origin".into(),
            refspec: "refs/pull/127/head:refs/heads/pr-127".into(),
            branch: "pr-127".into(),
            second_parent: false,
        }
    );
    let gl = Some(PullFetch::Head("refs/merge-requests/127/head".into()));
    let plan = fetch_plan(&p, "origin", "https://gitlab.com", gl, "MR", false).unwrap();
    assert_eq!(
        plan.refspec,
        "refs/merge-requests/127/head:refs/heads/mr-127"
    );
    assert_eq!(plan.branch, "mr-127");
    // Azure DevOps: its merge, whose second parent is its head.
    let az = Some(PullFetch::Merge("refs/pull/127/merge".into()));
    let plan = fetch_plan(&p, "origin", "https://dev.azure.com/acme", az, "PR", false).unwrap();
    assert_eq!(plan.refspec, "refs/pull/127/merge");
    assert!(plan.second_parent);
    // Bitbucket Cloud: from the fork itself, on the same instance.
    let plan = fetch_plan(
        &p,
        "origin",
        "https://bitbucket.org/",
        Some(PullFetch::ForkBranch),
        "PR",
        false,
    )
    .unwrap();
    assert_eq!(plan.from, "https://bitbucket.org/marta/demo.git");
    assert_eq!(plan.refspec, "refs/heads/i18n-es:refs/heads/pr-127");
}

#[test]
fn a_forks_pull_request_its_host_cannot_give_is_refused() {
    let p = pr(9, "fix", true, "");
    assert!(fetch_plan(&p, "origin", "https://x", None, "PR", false).is_err());
    // Bitbucket without the fork's repository.
    assert!(fetch_plan(
        &p,
        "origin",
        "https://x",
        Some(PullFetch::ForkBranch),
        "PR",
        false
    )
    .is_err());
    // A branch that would read as an option.
    assert!(fetch_plan(
        &pr(9, "-x", false, ""),
        "origin",
        "https://x",
        None,
        "PR",
        false
    )
    .is_err());
    assert!(fetch_plan(
        &pr(9, "", false, ""),
        "origin",
        "https://x",
        None,
        "PR",
        false
    )
    .is_err());
}

#[test]
fn the_local_branch_of_a_fork_says_the_hosts_word() {
    assert_eq!(fork_branch("PR", 5), "pr-5");
    assert_eq!(fork_branch("MR", 12), "mr-12");
}

#[test]
fn each_host_says_how_a_forks_pull_request_is_fetched() {
    use crate::forge::Registry;
    let r = Registry::standard();
    let of = |k: &str| r.get(k).unwrap().pull_fetch(3);
    assert_eq!(
        of("github"),
        Some(PullFetch::Head("refs/pull/3/head".into()))
    );
    assert_eq!(
        of("gitea"),
        Some(PullFetch::Head("refs/pull/3/head".into()))
    );
    assert_eq!(
        of("gitlab"),
        Some(PullFetch::Head("refs/merge-requests/3/head".into()))
    );
    assert_eq!(
        of("azure"),
        Some(PullFetch::Merge("refs/pull/3/merge".into()))
    );
    assert_eq!(of("bitbucket"), Some(PullFetch::ForkBranch));
}

/// On real repositories: a pull request's ref fetched into `pr-<n>`, and its merge's second parent.
mod repos {
    use crate::git;
    use crate::pragent::FetchPlan;
    use std::process::Command;

    fn g(dir: &std::path::Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// An « origin » with `main`, a pull request's head kept under refs/pull/7/head and its merge
    /// under refs/pull/7/merge; and a clone of it.
    fn origin_and_clone(name: &str) -> (std::path::PathBuf, String) {
        let t = crate::paths::test_dir(name);
        let o = t.join("origin");
        std::fs::create_dir_all(&o).unwrap();
        g(&o, &["init", "-q", "-b", "main"]);
        g(&o, &["config", "user.email", "t@t"]);
        g(&o, &["config", "user.name", "t"]);
        g(&o, &["config", "core.autocrlf", "false"]);
        std::fs::write(o.join("a"), "1").unwrap();
        g(&o, &["add", "."]);
        g(&o, &["commit", "-qm", "one"]);
        g(&o, &["checkout", "-qb", "fork-work"]);
        std::fs::write(o.join("b"), "2").unwrap();
        g(&o, &["add", "."]);
        g(&o, &["commit", "-qm", "the fork's"]);
        let head = g(&o, &["rev-parse", "HEAD"]);
        g(&o, &["update-ref", "refs/pull/7/head", &head]);
        g(&o, &["checkout", "-q", "main"]);
        g(&o, &["merge", "-q", "--no-ff", "-m", "merge", "fork-work"]);
        let merge = g(&o, &["rev-parse", "HEAD"]);
        g(&o, &["update-ref", "refs/pull/7/merge", &merge]);
        g(&o, &["reset", "-q", "--hard", "HEAD~1"]);
        g(&o, &["branch", "-qD", "fork-work"]);
        let c = t.join("clone");
        g(
            &t,
            &["clone", "-q", o.to_str().unwrap(), c.to_str().unwrap()],
        );
        (c, head)
    }

    #[tokio::test]
    async fn a_forks_head_is_fetched_into_its_branch() {
        let (c, head) = origin_and_clone("pragent-head");
        let plan = FetchPlan {
            from: "origin".into(),
            refspec: "refs/pull/7/head:refs/heads/pr-7".into(),
            branch: "pr-7".into(),
            second_parent: false,
        };
        let root = c.to_str().unwrap();
        git::fetch_ref(root, &plan.from, &plan.refspec)
            .await
            .unwrap();
        assert_eq!(
            git::branch_head(root, "pr-7").await.as_deref(),
            Some(head.as_str())
        );
    }

    #[tokio::test]
    async fn the_second_parent_of_a_merge_is_its_head() {
        let (c, head) = origin_and_clone("pragent-merge");
        let root = c.to_str().unwrap();
        git::fetch_ref(root, "origin", "refs/pull/7/merge")
            .await
            .unwrap();
        git::run(root, &["branch", "-f", "pr-7", "FETCH_HEAD^2"])
            .await
            .unwrap();
        assert_eq!(
            git::branch_head(root, "pr-7").await.as_deref(),
            Some(head.as_str())
        );
    }

    /// The fork's pull request by its head (GitHub, GitLab) and by its merge (Azure DevOps).
    fn plans() -> [FetchPlan; 2] {
        [
            FetchPlan {
                from: "origin".into(),
                refspec: "refs/pull/7/head:refs/heads/pr-7".into(),
                branch: "pr-7".into(),
                second_parent: false,
            },
            FetchPlan {
                from: "origin".into(),
                refspec: "refs/pull/7/merge".into(),
                branch: "pr-7".into(),
                second_parent: true,
            },
        ]
    }

    #[tokio::test]
    async fn a_forks_branch_here_with_commits_of_its_own_is_never_overwritten() {
        for (i, plan) in plans().iter().enumerate() {
            let (c, head) = origin_and_clone(&format!("pragent-own-{i}"));
            let root = c.to_str().unwrap();
            // An earlier agent's work on pr-7, its worktree gone: a commit the pull request has not.
            g(&c, &["config", "user.email", "t@t"]);
            g(&c, &["config", "user.name", "t"]);
            g(&c, &["checkout", "-qb", "pr-7"]);
            std::fs::write(c.join("mine"), "3").unwrap();
            g(&c, &["add", "."]);
            g(&c, &["commit", "-qm", "an agent's"]);
            let mine = g(&c, &["rev-parse", "HEAD"]);
            g(&c, &["checkout", "-q", "main"]);
            crate::pragent::fetch_pr(root, plan).await.unwrap();
            let now = git::branch_head(root, "pr-7").await;
            assert_eq!(now.as_deref(), Some(mine.as_str()), "{plan:?}");
            assert_ne!(now.as_deref(), Some(head.as_str()));
        }
    }

    #[tokio::test]
    async fn a_forks_branch_here_behind_the_pull_request_moves_forward() {
        for (i, plan) in plans().iter().enumerate() {
            let (c, head) = origin_and_clone(&format!("pragent-behind-{i}"));
            let root = c.to_str().unwrap();
            // pr-7 fetched before the fork's last commit: where the pull request began.
            g(&c, &["branch", "pr-7", "main"]);
            crate::pragent::fetch_pr(root, plan).await.unwrap();
            assert_eq!(
                git::branch_head(root, "pr-7").await.as_deref(),
                Some(head.as_str()),
                "{plan:?}"
            );
        }
    }
}

/// On the real core against a fake GitHub: an agent on a pull request's own branch, then on a
/// fork's, in a worktree on the branch fetched.
mod core {
    use crate::agentprs_tests::github_project;
    use crate::core_tests::{git, harness};
    use crate::integrations::fake::FakeServer;
    use serde_json::json;

    fn node(number: u64, head: &str, fork: bool) -> serde_json::Value {
        json!({
            "number": number, "url": format!("https://github.com/acme/demo/pull/{number}"), "title": "t",
            "state": "OPEN", "isDraft": false, "headRefName": head, "baseRefName": "main",
            "mergeable": "MERGEABLE", "author": { "login": "bob" }, "createdAt": "2026-10-29T08:00:00Z",
            "isCrossRepository": fork, "headRepository": { "nameWithOwner": if fork { "bob/demo" } else { "acme/demo" } },
            "autoMergeRequest": null, "latestOpinionatedReviews": { "nodes": [] }, "reviewThreads": { "nodes": [] },
            "commits": { "nodes": [] },
        })
    }

    #[tokio::test]
    async fn an_agent_works_on_the_branch_of_a_pull_request_its_own_or_a_forks() {
        let h = harness("pragent-core");
        let server = FakeServer::start().await;
        let (p, r, bare) = github_project(&h, &server, true).await;
        // Someone's branch on the remote, and a fork's pull request kept by the remote as refs/pull/8/head.
        git(&r, &["checkout", "-qb", "feat/login"]);
        std::fs::write(r.join("login.txt"), "x").unwrap();
        git(&r, &["add", "."]);
        git(&r, &["commit", "-qm", "login"]);
        git(&r, &["push", "-q", "origin", "feat/login"]);
        let own = git(&r, &["rev-parse", "HEAD"]);
        git(&r, &["checkout", "-qb", "fork-work", "main"]);
        std::fs::write(r.join("fork.txt"), "y").unwrap();
        git(&r, &["add", "."]);
        git(&r, &["commit", "-qm", "fork"]);
        let forked = git(&r, &["rev-parse", "HEAD"]);
        git(&r, &["push", "-q", "origin", "fork-work:refs/pull/8/head"]);
        git(&r, &["checkout", "-q", "main"]);
        git(&r, &["branch", "-qD", "feat/login", "fork-work"]);
        let _ = bare;
        server.on(
            "POST",
            "/graphql#OpenPulls",
            200,
            json!({ "data": { "repository": { "pullRequests": { "nodes": [node(7, "feat/login", false), node(8, "fix", true)] } } } }),
        );

        let a = h.core.create_agent_on_pr(&p.id, 7).await.unwrap();
        let wt = a.meta.worktree.clone().expect("a worktree");
        assert_eq!(wt.branch, "feat/login");
        assert_eq!(
            git(std::path::Path::new(&wt.path), &["rev-parse", "HEAD"]),
            own
        );

        let b = h.core.create_agent_on_pr(&p.id, 8).await.unwrap();
        let wt = b.meta.worktree.clone().expect("a worktree");
        assert_eq!(wt.branch, "pr-8");
        assert_eq!(
            git(std::path::Path::new(&wt.path), &["rev-parse", "HEAD"]),
            forked
        );

        // Taken already: refused as any branch another worktree has; and one no longer open.
        assert!(h.core.create_agent_on_pr(&p.id, 8).await.is_err());
        let e = h.core.create_agent_on_pr(&p.id, 99).await.unwrap_err();
        assert_eq!(e.to_string(), "La PR #99 n'est plus ouverte.");
    }
}
