//! What the view of a pull request writes on its host (ESC-474): the writes checked before they are asked, by the host's
//! capabilities, against a fake GitHub served by the plugin « GitHub », their failures in the window's words, and none
//! of them reachable from the MCP server.

use crate::agentprs_tests::github_project;
use crate::core_tests::harness;
use crate::forge::{DiffSide, Writing};
use crate::integrations::fake::FakeServer;
use crate::integrations::HttpError;
use crate::prread_tests::routes;
use crate::prview::FailureKind;
use crate::prwrite::*;
use serde_json::{json, Value};

fn line(body: &str) -> PrWrite {
    PrWrite::LineComment {
        path: "lien.ts".into(),
        line: 2,
        start_line: Some(1),
        side: DiffSide::Right,
        body: body.into(),
    }
}

#[test]
fn a_write_without_text_or_without_a_place_is_refused_before_it_is_asked() {
    assert!(line("Pourquoi ?").check().is_ok());
    assert!(line("  \n").check().is_err(), "an empty text");
    assert!(line(&"x".repeat(BODY_MAX + 1)).check().is_err());
    let backwards = PrWrite::LineComment {
        path: "a".into(),
        line: 2,
        start_line: Some(5),
        side: DiffSide::Right,
        body: "x".into(),
    };
    assert!(backwards.check().is_err(), "a range upside down");
    assert!(PrWrite::Resolve { thread: "".into() }.check().is_err());
    assert!(PrWrite::Reply {
        comment: "".into(),
        body: "x".into()
    }
    .check()
    .is_err());
    // What the window sends, in its words.
    let sent: PrWrite = serde_json::from_value(
        json!({ "kind": "lineComment", "path": "a", "line": 3, "startLine": 1, "side": "left", "body": "x" }),
    )
    .unwrap();
    assert!(matches!(
        sent,
        PrWrite::LineComment {
            side: DiffSide::Left,
            start_line: Some(1),
            ..
        }
    ));
    let reopen: PrWrite =
        serde_json::from_value(json!({ "kind": "reopen", "thread": "T" })).unwrap();
    assert_eq!(reopen, PrWrite::Reopen { thread: "T".into() });
}

#[test]
fn a_write_is_offered_by_the_hosts_capability() {
    let answers = Writing::ANSWERS;
    assert!(PrWrite::Reply {
        comment: "1".into(),
        body: "x".into()
    }
    .allowed(&answers));
    assert!(!line("x").allowed(&answers), "no comment on a line");
    assert!(!PrWrite::Reopen { thread: "T".into() }.allowed(&answers));
    assert!(PrWrite::Reopen { thread: "T".into() }.allowed(&Writing::ALL));
    assert!(!PrWrite::Comment { body: "x".into() }.allowed(&Writing::default()));
}

#[test]
fn a_refused_write_says_the_right_to_write_and_a_thread_gone_apart() {
    let http = |status: u16, message: &str| {
        anyhow::Error::new(HttpError {
            status,
            message: message.into(),
        })
    };
    assert_eq!(
        write_failure(
            &http(403, "Resource not accessible by personal access token"),
            None
        )
        .kind,
        FailureKind::Forbidden
    );
    assert_eq!(
        write_failure(&http(404, "Not Found"), None).kind,
        FailureKind::Gone
    );
    assert_eq!(
        write_failure(&http(401, "Bad credentials"), None).kind,
        FailureKind::Refused
    );
    assert_eq!(
        write_failure(&http(403, "API rate limit exceeded"), None).kind,
        FailureKind::RateLimit
    );
    assert_eq!(
        write_failure(&anyhow::anyhow!("connexion refusée"), None).kind,
        FailureKind::Unreachable,
        "no answer: out of reach (offline)"
    );
}

/// The writes that reached GitHub: their path, and their operation for GraphQL.
fn writes(server: &FakeServer) -> Vec<(String, Value)> {
    server
        .writes()
        .into_iter()
        .map(|r| {
            let body = r.json();
            let at = match r.path() {
                "/graphql" => format!("graphql#{}", body["operationName"].as_str().unwrap_or("")),
                p => p.to_string(),
            };
            (at, body)
        })
        .filter(|(at, _)| !at.starts_with("graphql#Pulls") && !at.starts_with("graphql#PrThreads"))
        .collect()
}

#[tokio::test]
async fn each_write_goes_to_github_in_the_users_name_and_the_pull_request_is_read_again() {
    let h = harness("prwrite-github");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    routes(&server, "2026-11-14T10:00:00Z");
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/comments",
        201,
        json!({ "id": 20 }),
    );
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/comments/1/replies",
        201,
        json!({ "id": 21 }),
    );
    server.on(
        "POST",
        "/repos/acme/demo/issues/12/comments",
        201,
        json!({ "id": 22 }),
    );
    server.on(
        "POST",
        "/graphql#ResolveThread",
        200,
        json!({ "data": { "resolveReviewThread": { "thread": { "id": "T1", "isResolved": true } } } }),
    );
    server.on(
        "POST",
        "/graphql#UnresolveThread",
        200,
        json!({ "data": { "unresolveReviewThread": { "thread": { "id": "T2", "isResolved": false } } } }),
    );
    let read = h.core.pr_contents(&p.id, 12, "", false).await;
    let c = read.data.expect("read");
    assert_eq!(c.writing, Writing::ALL, "GitHub writes all five");
    assert!(!h.core.pr_contents.lock().values().any(|k| k.stale));

    let done = |r: crate::prview::Read<bool>| {
        assert!(r.failure.is_none(), "{:?}", r.failure);
        assert_eq!(r.data, Some(true));
    };
    done(h.core.pr_write(&p.id, 12, "", line("Un test ici ?")).await);
    assert!(
        h.core.pr_contents.lock().values().all(|k| k.stale),
        "written: read again at the next ask"
    );
    done(
        h.core
            .pr_write(
                &p.id,
                12,
                "",
                PrWrite::Reply {
                    comment: "1".into(),
                    body: "Ajouté.".into(),
                },
            )
            .await,
    );
    done(
        h.core
            .pr_write(
                &p.id,
                12,
                "",
                PrWrite::Resolve {
                    thread: "T1".into(),
                },
            )
            .await,
    );
    done(
        h.core
            .pr_write(
                &p.id,
                12,
                "",
                PrWrite::Reopen {
                    thread: "T2".into(),
                },
            )
            .await,
    );
    done(
        h.core
            .pr_write(
                &p.id,
                12,
                "",
                PrWrite::Comment {
                    body: "Merci !".into(),
                },
            )
            .await,
    );

    let w = writes(&server);
    let at: Vec<&str> = w.iter().map(|(a, _)| a.as_str()).collect();
    assert_eq!(
        at,
        [
            "/repos/acme/demo/pulls/12/comments",
            "/repos/acme/demo/pulls/12/comments/1/replies",
            "graphql#ResolveThread",
            "graphql#UnresolveThread",
            "/repos/acme/demo/issues/12/comments",
        ]
    );
    let comment = &w[0].1;
    assert_eq!(
        (
            &comment["path"],
            &comment["line"],
            &comment["start_line"],
            &comment["side"]
        ),
        (&json!("lien.ts"), &json!(2), &json!(1), &json!("RIGHT"))
    );
    assert_eq!(
        comment["commit_id"], "abc",
        "the tip, as GitHub says it when no lines are kept"
    );
    assert_eq!(w[1].1["body"], "Ajouté.");
    assert_eq!(w[2].1["variables"]["id"], "T1");
    assert_eq!(w[3].1["variables"]["id"], "T2");
}

#[tokio::test]
async fn a_write_the_host_refuses_is_said_in_the_windows_words() {
    let h = harness("prwrite-refused");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    routes(&server, "2026-11-14T10:00:00Z");
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/comments/1/replies",
        403,
        json!({ "message": "Resource not accessible by personal access token" }),
    );
    server.on(
        "POST",
        "/graphql#ResolveThread",
        200,
        json!({ "data": { "resolveReviewThread": null },
                "errors": [{ "type": "NOT_FOUND", "message": "Could not resolve to a node with the global id of 'T9'" }] }),
    );
    let reply = h
        .core
        .pr_write(
            &p.id,
            12,
            "",
            PrWrite::Reply {
                comment: "1".into(),
                body: "Ajouté.".into(),
            },
        )
        .await;
    let f = reply.failure.expect("refused");
    assert_eq!(f.kind, FailureKind::Forbidden, "{}", f.message);
    assert!(f.message.contains("Resource not accessible"));
    let gone = h
        .core
        .pr_write(
            &p.id,
            12,
            "",
            PrWrite::Resolve {
                thread: "T9".into(),
            },
        )
        .await;
    assert_eq!(gone.failure.expect("gone").kind, FailureKind::Gone);
    // Nothing asked for an empty text.
    let before = server.writes().len();
    let empty = h
        .core
        .pr_write(&p.id, 12, "", PrWrite::Comment { body: " ".into() })
        .await;
    assert_eq!(empty.failure.expect("empty").kind, FailureKind::Other);
    assert_eq!(server.writes().len(), before);
}

#[test]
fn no_tool_of_the_mcp_server_writes_on_a_pull_request() {
    // Only the window's commands call `pr_write` and `pr_merge`: an agent, its tools or anything by itself never
    // comments, answers, resolves, changes a draft, asks a review, labels, updates a branch, runs a check again, turns
    // the auto-merge on or off, merges nor deletes a branch in the user's name (ESC-474, ESC-488).
    let sources = [
        ("mcp/prs.rs", include_str!("mcp/prs.rs")),
        ("mcp/tools.rs", include_str!("mcp/tools.rs")),
        ("mcp/act.rs", include_str!("mcp/act.rs")),
        ("mcp/mod.rs", include_str!("mcp/mod.rs")),
        ("mcp/activity.rs", include_str!("mcp/activity.rs")),
        ("mcp/asks.rs", include_str!("mcp/asks.rs")),
        ("mcp/background.rs", include_str!("mcp/background.rs")),
        // The browsers and the agents' pages (ESC-669 to ESC-685, reversed by ESC-701).
        ("mcp/browsers.rs", include_str!("mcp/browsers.rs")),
        (
            "mcp/browsers_drive.rs",
            include_str!("mcp/browsers_drive.rs"),
        ),
        ("mcp/browsers_open.rs", include_str!("mcp/browsers_open.rs")),
        ("mcp/pages.rs", include_str!("mcp/pages.rs")),
        ("mcp/docs.rs", include_str!("mcp/docs.rs")),
        ("mcp/files.rs", include_str!("mcp/files.rs")),
        ("mcp/http.rs", include_str!("mcp/http.rs")),
        ("mcp/images.rs", include_str!("mcp/images.rs")),
        ("mcp/install.rs", include_str!("mcp/install.rs")),
        ("mcp/memory.rs", include_str!("mcp/memory.rs")),
        ("mcp/overseer.rs", include_str!("mcp/overseer.rs")),
        ("mcp/read.rs", include_str!("mcp/read.rs")),
        ("mcp/resolve.rs", include_str!("mcp/resolve.rs")),
        ("mcp/scheduled.rs", include_str!("mcp/scheduled.rs")),
        ("mcp/wait.rs", include_str!("mcp/wait.rs")),
        ("mcp/workflow.rs", include_str!("mcp/workflow.rs")),
    ];
    let writes = [
        "pr_write",
        "PrWrite",
        "pr_merge",
        "MergeAsk",
        ".unresolve(",
        ".line_comment(",
        ".set_draft(",
        ".request_reviewers(",
        ".remove_reviewer(",
        ".add_labels(",
        ".remove_label(",
        ".update_branch(",
        ".rerun_check(",
        ".cancel_auto_merge(",
        ".delete_branch(",
        ".auto_merge(",
        ".merge(",
        "ticket_pr_merge",
    ];
    for (name, source) in sources {
        for call in writes {
            assert!(!source.contains(call), "{name} calls {call}");
        }
    }
    // Every file of the server is looked at: a new one is added above.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mcp");
    for entry in std::fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().to_string();
        if name.ends_with(".rs") && !name.ends_with("_tests.rs") && name != "tests.rs" {
            assert!(
                sources.iter().any(|(n, _)| *n == format!("mcp/{name}")),
                "mcp/{name} is not looked at"
            );
        }
    }
}

// ---------- The control of a pull request (ESC-488) ----------

#[test]
fn the_controls_are_offered_by_the_hosts_capabilities_and_checked_before_they_are_asked() {
    use crate::forge::{MergeStrategy, UpdateMethod};
    use crate::prview::Can;
    let all = [
        PrWrite::SetDraft { draft: false },
        PrWrite::RequestReviewers {
            logins: vec!["marta".into()],
        },
        PrWrite::RemoveReviewer {
            login: "bob".into(),
        },
        PrWrite::AddLabels {
            labels: vec!["bug".into()],
        },
        PrWrite::RemoveLabel {
            label: "bug".into(),
        },
        PrWrite::UpdateBranch {
            method: UpdateMethod::Merge,
        },
        PrWrite::UpdateBranch {
            method: UpdateMethod::Rebase,
        },
        PrWrite::RerunCheck { job: "42".into() },
        PrWrite::CancelAutoMerge,
    ];
    for w in &all {
        assert!(w.check().is_ok(), "{w:?}");
        assert!(w.allowed(&Writing::ALL), "GitHub: {w:?}");
        assert!(!w.allowed(&Writing::ANSWERS), "the others: {w:?}");
    }
    // The auto-merge is the host's own, with a strategy it merges with.
    let on = PrWrite::AutoMerge {
        strategy: MergeStrategy::Squash,
    };
    let mut can = Can {
        writing: Writing::ALL,
        merge_strategies: vec![MergeStrategy::Merge],
        auto_merge: true,
        ..Default::default()
    };
    assert!(!on.allowed_in(&can), "squash is not among its strategies");
    can.merge_strategies.push(MergeStrategy::Squash);
    assert!(on.allowed_in(&can));
    can.auto_merge = false;
    assert!(!on.allowed_in(&can));
    // Nothing unnamed.
    for w in [
        PrWrite::RequestReviewers { logins: vec![] },
        PrWrite::RequestReviewers {
            logins: vec![" ".into()],
        },
        PrWrite::AddLabels { labels: vec![] },
        PrWrite::RemoveLabel { label: "".into() },
        PrWrite::RemoveReviewer { login: " ".into() },
        PrWrite::RerunCheck { job: "".into() },
    ] {
        assert!(w.check().is_err(), "{w:?}");
    }
    // As the window sends them.
    let sent: PrWrite =
        serde_json::from_value(json!({ "kind": "updateBranch", "method": "rebase" })).unwrap();
    assert_eq!(
        sent,
        PrWrite::UpdateBranch {
            method: UpdateMethod::Rebase
        }
    );
    let off: PrWrite = serde_json::from_value(json!({ "kind": "cancelAutoMerge" })).unwrap();
    assert_eq!(off, PrWrite::CancelAutoMerge);
    let ask: MergeAsk =
        serde_json::from_value(json!({ "strategy": "squash", "deleteBranch": true })).unwrap();
    assert!(ask.delete_branch && !ask.delete_worktree);
}

#[test]
fn a_refused_merge_says_a_conflict_the_rules_and_a_check_required_apart() {
    let http = |status: u16, message: &str| {
        anyhow::Error::new(HttpError {
            status,
            message: message.into(),
        })
    };
    let kind = |status, message| write_failure(&http(status, message), None).kind;
    assert_eq!(
        kind(405, "Required status check \"e2e\" is expected."),
        FailureKind::CheckRequired
    );
    assert_eq!(
        kind(
            405,
            "At least 1 approving review is required by reviewers with write access."
        ),
        FailureKind::Protected
    );
    assert_eq!(
        kind(405, "Repository rule violations found"),
        FailureKind::Protected
    );
    assert_eq!(
        kind(405, "Pull Request is not mergeable"),
        FailureKind::Conflict
    );
    assert_eq!(
        kind(
            409,
            "Head branch was modified. Review and try the merge again."
        ),
        FailureKind::Conflict
    );
    assert_eq!(
        kind(
            422,
            "GitHub (GraphQL) : merge conflict between base and head"
        ),
        FailureKind::Conflict
    );
    assert_eq!(
        kind(403, "Resource not accessible by integration"),
        FailureKind::Forbidden
    );
    assert_eq!(kind(403, "API rate limit exceeded"), FailureKind::RateLimit);
    assert_eq!(kind(422, "Validation Failed"), FailureKind::Other);
}

/// The routes of the control on the fake GitHub, beside `routes`'.
fn control_routes(server: &FakeServer) {
    server.on(
        "GET",
        "/repos/acme/demo",
        200,
        json!({ "allow_merge_commit": true, "allow_squash_merge": true, "allow_rebase_merge": false,
                "allow_auto_merge": true }),
    );
    server.on(
        "POST",
        "/graphql#PrControl",
        200,
        json!({ "data": { "repository": { "deleteBranchOnMerge": false, "pullRequest": {
            "id": "PR_12", "mergeStateStatus": "BEHIND", "reviewDecision": "REVIEW_REQUIRED",
            "baseRefOid": "def", "headRefOid": "abc", "labels": { "nodes": [{ "name": "auth" }] },
            "autoMergeRequest": null } } } }),
    );
    server.on(
        "GET",
        "/repos/acme/demo/compare/def...abc",
        200,
        json!({ "behind_by": 2 }),
    );
    server.on(
        "POST",
        "/graphql#PrCandidates",
        200,
        json!({ "data": { "repository": { "assignableUsers": { "nodes": [{ "login": "marta" }] },
                "labels": { "nodes": [{ "name": "auth", "color": "0e8a16" }, { "name": "bug" }] } } } }),
    );
    server.on(
        "POST",
        "/graphql#PullRequestId",
        200,
        json!({ "data": { "repository": { "pullRequest": { "id": "PR_12" } } } }),
    );
    for (op, field, extra) in [
        (
            "MarkReady",
            "markPullRequestReadyForReview",
            json!({ "isDraft": false }),
        ),
        (
            "UpdateBranch",
            "updatePullRequestBranch",
            json!({ "number": 12 }),
        ),
        (
            "EnableAutoMerge",
            "enablePullRequestAutoMerge",
            json!({ "number": 12 }),
        ),
        (
            "DisableAutoMerge",
            "disablePullRequestAutoMerge",
            json!({ "number": 12 }),
        ),
    ] {
        server.on(
            "POST",
            &format!("/graphql#{op}"),
            200,
            json!({ "data": { field: { "pullRequest": extra } } }),
        );
    }
    for (method, path) in [
        ("POST", "/repos/acme/demo/pulls/12/requested_reviewers"),
        ("DELETE", "/repos/acme/demo/pulls/12/requested_reviewers"),
        ("POST", "/repos/acme/demo/issues/12/labels"),
        ("DELETE", "/repos/acme/demo/issues/12/labels/auth"),
        ("POST", "/repos/acme/demo/actions/jobs/77/rerun"),
    ] {
        server.on(method, path, 200, json!({}));
    }
}

#[tokio::test]
async fn the_merge_box_reads_where_the_pull_request_stands_and_the_kanbans_strategy() {
    let h = harness("prcontrol-read");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    routes(&server, "2026-11-14T10:00:00Z");
    control_routes(&server);
    h.set_board(&p.id, |s| s.strategy = "rebase".into());
    let read = h.core.pr_control(&p.id, 12, "").await;
    let r = read.data.unwrap_or_else(|| panic!("{:?}", read.failure));
    let c = r.control.expect("GitHub reads it");
    assert_eq!(c.merge_state, crate::forge::MergeState::Behind);
    assert_eq!(c.behind, Some(true));
    assert_eq!(
        c.review_decision,
        Some(crate::forge::ReviewDecision::ReviewRequired)
    );
    use crate::forge::MergeStrategy::*;
    assert_eq!(r.options.strategies, vec![Merge, Squash]);
    assert!(r.options.auto_merge);
    assert_eq!(
        r.preferred,
        Some(Merge),
        "rebase is not allowed: the first allowed"
    );
    assert_eq!(r.writing, Writing::ALL);
    let k = h
        .core
        .pr_candidates(&p.id, 12, "")
        .await
        .data
        .expect("read");
    assert_eq!(k.reviewers, vec!["marta".to_string()]);
    assert_eq!(k.labels.len(), 2);
    // Reading writes nothing.
    assert!(
        writes(&server)
            .iter()
            .all(|(at, _)| at.starts_with("graphql#Pr")),
        "{:?}",
        writes(&server)
    );
}

#[tokio::test]
async fn each_control_goes_to_github_in_the_users_name() {
    use crate::forge::{MergeStrategy, UpdateMethod};
    let h = harness("prcontrol-write");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    routes(&server, "2026-11-14T10:00:00Z");
    control_routes(&server);
    let asks = [
        PrWrite::SetDraft { draft: false },
        PrWrite::RequestReviewers {
            logins: vec!["marta".into()],
        },
        PrWrite::RemoveReviewer {
            login: "bob".into(),
        },
        PrWrite::AddLabels {
            labels: vec!["bug".into()],
        },
        PrWrite::RemoveLabel {
            label: "auth".into(),
        },
        PrWrite::UpdateBranch {
            method: UpdateMethod::Merge,
        },
        PrWrite::RerunCheck { job: "77".into() },
        PrWrite::AutoMerge {
            strategy: MergeStrategy::Squash,
        },
        PrWrite::CancelAutoMerge,
    ];
    for w in asks {
        let r = h.core.pr_write(&p.id, 12, "", w.clone()).await;
        assert!(r.failure.is_none(), "{w:?}: {:?}", r.failure);
    }
    let at: Vec<String> = writes(&server)
        .into_iter()
        .map(|(a, _)| a)
        .filter(|a| a != "graphql#PullRequestId")
        .collect();
    assert_eq!(
        at,
        [
            "graphql#MarkReady",
            "/repos/acme/demo/pulls/12/requested_reviewers",
            "/repos/acme/demo/pulls/12/requested_reviewers",
            "/repos/acme/demo/issues/12/labels",
            "/repos/acme/demo/issues/12/labels/auth",
            "graphql#UpdateBranch",
            "/repos/acme/demo/actions/jobs/77/rerun",
            "graphql#EnableAutoMerge",
            "graphql#DisableAutoMerge",
        ]
    );
    let auto: Vec<Value> = server
        .writes()
        .into_iter()
        .map(|r| r.json())
        .filter(|b| b["operationName"] == "EnableAutoMerge")
        .collect();
    assert_eq!(auto[0]["variables"]["method"], "SQUASH");
}

#[tokio::test]
async fn a_control_github_refuses_is_said_and_nothing_is_asked_before_its_check() {
    use crate::forge::UpdateMethod;
    let h = harness("prcontrol-refused");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    routes(&server, "2026-11-14T10:00:00Z");
    control_routes(&server);
    server.on(
        "POST",
        "/graphql#UpdateBranch",
        200,
        json!({ "data": { "updatePullRequestBranch": null },
                "errors": [{ "type": "UNPROCESSABLE", "message": "merge conflict between base and head" }] }),
    );
    server.on(
        "POST",
        "/repos/acme/demo/issues/12/labels",
        403,
        json!({ "message": "Resource not accessible by personal access token" }),
    );
    let update = h
        .core
        .pr_write(
            &p.id,
            12,
            "",
            PrWrite::UpdateBranch {
                method: UpdateMethod::Merge,
            },
        )
        .await;
    assert_eq!(update.failure.expect("refused").kind, FailureKind::Conflict);
    let label = h
        .core
        .pr_write(
            &p.id,
            12,
            "",
            PrWrite::AddLabels {
                labels: vec!["bug".into()],
            },
        )
        .await;
    assert_eq!(label.failure.expect("refused").kind, FailureKind::Forbidden);
    let before = server.writes().len();
    let none = h
        .core
        .pr_write(&p.id, 12, "", PrWrite::AddLabels { labels: vec![] })
        .await;
    assert!(none.failure.is_some());
    assert_eq!(server.writes().len(), before);
}
