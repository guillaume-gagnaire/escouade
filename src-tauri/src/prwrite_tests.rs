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
    // Only the window's command calls `pr_write`: an agent, its tools or anything by itself never comments, answers or
    // resolves in the user's name.
    for (name, source) in [
        ("mcp/prs.rs", include_str!("mcp/prs.rs")),
        ("mcp/tools.rs", include_str!("mcp/tools.rs")),
        ("mcp/act.rs", include_str!("mcp/act.rs")),
        ("mcp/mod.rs", include_str!("mcp/mod.rs")),
    ] {
        for call in ["pr_write", "PrWrite", ".unresolve(", ".line_comment("] {
            assert!(!source.contains(call), "{name} calls {call}");
        }
    }
}
