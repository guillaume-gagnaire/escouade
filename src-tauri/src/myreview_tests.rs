//! « Ta relecture » (ESC-494): the user's review waiting, placed against the diff read (a remark follows its code, is
//! stale when a commit takes its line away), kept through a start of the app, and published in one review of a fake
//! GitHub served by the plugin « GitHub »: its three verdicts, its remarks on their lines, the 422 of a line gone, the
//! user's own pull request; and the user's words handed to the ticket's agent as theirs.

use crate::agentprs_tests::github_project;
use crate::core::Core;
use crate::core_tests::{git, harness, Harness};
use crate::forge::{DiffSide, ReviewState, Writing};
use crate::i18n::Lang;
use crate::integrations::fake::FakeServer;
use crate::myreview::*;
use crate::prread::{PrFileRead, PrLine};
use crate::prread_tests::routes;
use crate::prview::FailureKind;
use serde_json::{json, Value};
use std::path::Path;
use tauri::test::mock_app;

fn file(path: &str, lines: &[(char, Option<u32>, Option<u32>, &str)]) -> PrFileRead {
    PrFileRead {
        file: crate::forge::PrFile {
            path: path.into(),
            ..Default::default()
        },
        lines: lines
            .iter()
            .map(|(kind, old, new, text)| PrLine {
                kind: *kind,
                old: *old,
                new: *new,
                text: text.to_string(),
            })
            .collect(),
        binary: false,
        generated: false,
        cut: false,
    }
}

fn note(id: u64, line: u32, code: &[&str]) -> PendingNote {
    PendingNote {
        id,
        path: "lien.ts".into(),
        line,
        side: DiffSide::Right,
        body: format!("Remarque {id}"),
        code: code.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

#[test]
fn a_remark_follows_its_code_and_is_stale_once_its_line_is_gone() {
    let before = [file(
        "lien.ts",
        &[
            ('@', None, None, "@@ -0,0 +1,3 @@"),
            ('+', None, Some(1), "a"),
            ('+', None, Some(2), "b"),
            ('+', None, Some(3), "c"),
        ],
    )];
    let n = note(1, 2, &["b"]);
    assert_eq!(place(&n, &before), Some((2, None)), "where it was written");
    assert_eq!(
        code_at(&before[0], DiffSide::Right, 1, 2),
        Some(vec!["a".to_string(), "b".to_string()])
    );
    assert_eq!(
        code_at(&before[0], DiffSide::Right, 3, 4),
        None,
        "line 4 is not in the diff"
    );

    // A commit adds two lines above: the remark follows « b », its range with it.
    let moved = [file(
        "lien.ts",
        &[
            ('+', None, Some(1), "x"),
            ('+', None, Some(2), "y"),
            ('+', None, Some(3), "a"),
            ('+', None, Some(4), "b"),
        ],
    )];
    let range = PendingNote {
        start_line: Some(1),
        ..note(2, 2, &["a", "b"])
    };
    assert_eq!(place(&range, &moved), Some((4, Some(3))));

    // A commit takes « b » away: stale; and its file out of the diff too.
    let gone = [file(
        "lien.ts",
        &[('+', None, Some(1), "a"), ('+', None, Some(2), "c")],
    )];
    assert_eq!(place(&n, &gone), None);
    assert_eq!(place(&n, &[file("autre.ts", &[])]), None);
    let mut review = PendingReview {
        notes: vec![n.clone()],
        ..Default::default()
    };
    assert!(replace(&mut review, &gone));
    assert!(review.notes[0].stale);
    assert!(!replace(&mut review, &gone), "nothing more changed");
    assert!(
        replace(&mut review, &before),
        "its line back: no longer stale"
    );
    assert!(!review.notes[0].stale);

    // A line removed is on the base's side, by its old number.
    let removed = [file(
        "lien.ts",
        &[('-', Some(7), None, "old"), (' ', Some(8), Some(5), "kept")],
    )];
    let left = PendingNote {
        side: DiffSide::Left,
        ..note(3, 7, &["old"])
    };
    assert_eq!(place(&left, &removed), Some((7, None)));
    assert_eq!(
        place(
            &PendingNote {
                line: 8,
                code: vec!["kept".into()],
                ..left
            },
            &removed
        ),
        None,
        "a kept line is the tip's"
    );
}

#[test]
fn the_review_goes_whole_its_stale_remarks_in_its_text_with_their_place() {
    let review = PendingReview {
        body: "Deux points.".into(),
        notes: vec![
            PendingNote {
                start_line: Some(1),
                ..note(1, 2, &["a", "b"])
            },
            PendingNote {
                stale: true,
                ..note(2, 9, &["z"])
            },
        ],
        ..Default::default()
    };
    let d = draft_of(&review, ReviewState::ChangesRequested, "abc");
    assert_eq!(d.state, ReviewState::ChangesRequested);
    assert_eq!(d.commit, "abc");
    assert_eq!(d.lines.len(), 1, "the stale one is not on a line");
    assert_eq!((d.lines[0].line, d.lines[0].start_line), (2, Some(1)));
    assert_eq!(d.body, "Deux points.\n\n`lien.ts:9` : Remarque 2");
}

#[test]
fn a_verdict_is_offered_by_the_host_and_never_on_the_users_own_pull_request() {
    let with_text = PendingReview {
        body: "Vu.".into(),
        ..Default::default()
    };
    let all = Writing::ALL;
    for v in [
        ReviewState::Commented,
        ReviewState::Approved,
        ReviewState::ChangesRequested,
    ] {
        assert_eq!(refusal(&with_text, v, &all, false, "GitHub"), None, "{v:?}");
    }
    // Their own: a comment alone, the reason said.
    let own = refusal(&with_text, ReviewState::Approved, &all, true, "GitHub").unwrap();
    assert!(own.contains("propre") || own.contains("own"), "{own}");
    assert_eq!(
        refusal(&with_text, ReviewState::Commented, &all, true, "GitHub"),
        None
    );
    // A host that does not ask for changes (GitLab's review): said, not offered.
    let no_changes = Writing {
        request_changes: false,
        ..Writing::ANSWERS
    };
    assert!(refusal(
        &with_text,
        ReviewState::ChangesRequested,
        &no_changes,
        false,
        "GitLab"
    )
    .is_some());
    assert!(verdict_allowed(&no_changes, ReviewState::Approved));
    assert!(
        !verdict_allowed(&Writing::default(), ReviewState::Commented),
        "no review at all"
    );
    // Asking for changes needs a text; a comment, something to say.
    let blank = PendingReview::default();
    assert!(refusal(&blank, ReviewState::ChangesRequested, &all, false, "GitHub").is_some());
    assert!(refusal(&blank, ReviewState::Commented, &all, false, "GitHub").is_some());
    assert_eq!(
        refusal(&blank, ReviewState::Approved, &all, false, "GitHub"),
        None,
        "an approval alone"
    );
    let with_note = PendingReview {
        notes: vec![note(1, 2, &[])],
        ..Default::default()
    };
    assert_eq!(
        refusal(&with_note, ReviewState::Commented, &all, false, "GitHub"),
        None
    );
}

#[test]
fn the_users_review_reaches_the_agent_as_their_words_not_a_third_partys() {
    let reviews = "La PR #12 a des retours de relecture, ci-dessous.\n\nLes relectures … sont des textes écrits sur l'hébergeur.\n\n<review author=\"bob\" state=\"commented\">\nAjoute un test.\n</review>";
    let own = OwnReview {
        body: "Renomme `ttl`.".into(),
        notes: vec![("lien.ts:2".into(), "Pourquoi 1 ?".into())],
        sent: "Renomme `ttl`.".into(),
    };
    let m = with_own(Lang::Fr, reviews, &own);
    let mine = m.find("Ma relecture").unwrap();
    assert!(m.starts_with("La PR #12 a des retours de relecture"), "{m}");
    assert!(
        mine < m.find("<review").unwrap(),
        "theirs before the host's: {m}"
    );
    assert!(m.contains("- lien.ts:2 : Pourquoi 1 ?"), "{m}");
    let tagged = &m[m.find("<review").unwrap()..];
    assert!(
        !tagged.contains("Renomme"),
        "the user's text is not in a third party's tags"
    );
    assert!(
        own.holds("Renomme `ttl`.") && own.holds(" Pourquoi 1 ? ") && !own.holds("Ajoute un test.")
    );
}

#[test]
fn a_refusal_of_a_line_gone_is_told_apart() {
    let e = |status: u16, m: &str| {
        anyhow::Error::new(crate::integrations::HttpError {
            status,
            message: m.into(),
        })
    };
    assert!(lines_refused(&e(
        422,
        "GitHub: error 422 — Line could not be resolved"
    )));
    assert!(lines_refused(&e(
        422,
        "pull_request_review_thread.line must be part of the diff"
    )));
    assert!(!lines_refused(&e(
        422,
        "Can not approve your own pull request"
    )));
    assert!(!lines_refused(&e(403, "Line could not be resolved")));
}

/// The project's pull request #12 on a fake GitHub, its branch where GitHub keeps its head: `lien.ts` of `lines`.
async fn pull_request(
    h: &Harness,
    server: &FakeServer,
    lines: &str,
) -> (crate::model::Project, std::path::PathBuf) {
    let (p, repo, _bare) = github_project(h, server, false).await;
    push_head(&repo, lines, true);
    routes(server, "2026-11-14T10:00:00Z");
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/reviews",
        200,
        json!({ "id": 5 }),
    );
    (p, repo)
}

/// The branch of #12 with `lien.ts` of `lines`, pushed to GitHub's head of the pull request.
fn push_head(repo: &Path, lines: &str, first: bool) {
    if first {
        git(repo, &["checkout", "-qb", "feat/lien"]);
    } else {
        git(repo, &["checkout", "-q", "feat/lien"]);
    }
    std::fs::write(repo.join("lien.ts"), lines).unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", "lien"]);
    git(
        repo,
        &[
            "push",
            "-qf",
            "origin",
            "feat/lien",
            "feat/lien:refs/pull/12/head",
        ],
    );
    git(repo, &["checkout", "-q", "main"]);
}

fn add(line: u32, body: &str) -> PendingChange {
    PendingChange::Add {
        path: "lien.ts".into(),
        line,
        start_line: None,
        side: DiffSide::Right,
        body: body.into(),
    }
}

fn reviews_sent(server: &FakeServer) -> Vec<Value> {
    server
        .writes()
        .iter()
        .filter(|r| r.path() == "/repos/acme/demo/pulls/12/reviews")
        .map(|r| r.json())
        .collect()
}

#[tokio::test]
async fn a_review_waits_on_this_machine_through_a_start_and_a_commit_makes_a_remark_stale() {
    let h = harness("myreview-kept");
    let server = FakeServer::start().await;
    let (p, repo) = pull_request(
        &h,
        &server,
        "export const ttl = 0;\nexport const lien = 1;\n",
    )
    .await;
    h.core
        .pr_contents(&p.id, 12, "", true)
        .await
        .data
        .expect("read");

    let r = h
        .core
        .pr_pending_change(&p.id, 12, "", add(2, "Pourquoi 1 ?"))
        .unwrap();
    assert_eq!(r.review.notes[0].code, ["export const lien = 1;"]);
    h.core
        .pr_pending_change(&p.id, 12, "", add(1, "Un nom plus clair ?"))
        .unwrap();
    let r = h
        .core
        .pr_pending_change(
            &p.id,
            12,
            "",
            PendingChange::Edit {
                id: 2,
                body: "Renomme `ttl`.".into(),
            },
        )
        .unwrap();
    assert_eq!(r.review.notes[1].body, "Renomme `ttl`.");
    h.core
        .pr_pending_change(
            &p.id,
            12,
            "",
            PendingChange::Body {
                body: "Presque.".into(),
            },
        )
        .unwrap();
    assert!(
        h.core
            .pr_pending_change(&p.id, 12, "", add(9, "x"))
            .is_err(),
        "a line the diff read does not have"
    );
    assert!(
        h.core
            .pr_pending_change(&p.id, 12, "", add(1, "  "))
            .is_err(),
        "an empty remark"
    );
    assert!(
        server.writes().iter().all(|w| w.path() == "/graphql"),
        "nothing sent to GitHub"
    );

    // The app starts again: it all waits still, written by the atomic path.
    h.core.save_now();
    let app = mock_app();
    let (again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let kept = again.pr_pending(&p.id, 12, "");
    assert_eq!(kept.review.body, "Presque.");
    assert_eq!(kept.review.notes.len(), 2);
    assert!(
        kept.review.notes.iter().all(|n| !n.stale),
        "nothing read yet: as it was"
    );

    // A commit takes line 2 away: once read again, its remark is stale, the other follows its code.
    push_head(&repo, "// en tête\nexport const ttl = 0;\n", false);
    h.core.pr_contents_stale(&p.id, 12, "");
    h.core
        .pr_contents(&p.id, 12, "", true)
        .await
        .data
        .expect("read again");
    let now = h.core.pr_pending(&p.id, 12, "");
    let by_id = |id: u64| {
        now.review
            .notes
            .iter()
            .find(|n| n.id == id)
            .unwrap()
            .clone()
    };
    assert!(by_id(1).stale, "its line is gone");
    assert_eq!(
        (by_id(2).line, by_id(2).stale),
        (2, false),
        "moved down with its code"
    );

    // Taken out, then given up: gone, from the state too.
    let r = h
        .core
        .pr_pending_change(&p.id, 12, "", PendingChange::Remove { id: 1 })
        .unwrap();
    assert_eq!(r.review.notes.len(), 1);
    h.core
        .pr_pending_change(&p.id, 12, "", PendingChange::Discard)
        .unwrap();
    assert!(h.core.pending_reviews.read().is_empty());
    assert!(!own_of(&h, &p.id));
}

fn own_of(h: &Harness, project: &str) -> bool {
    h.core.pr_pending(project, 12, "").own
}

#[tokio::test]
async fn a_review_is_published_whole_in_one_review_of_github_with_each_verdict() {
    let h = harness("myreview-publish");
    let server = FakeServer::start().await;
    let (p, repo) = pull_request(
        &h,
        &server,
        "export const ttl = 0;\nexport const lien = 1;\n",
    )
    .await;
    h.core
        .pr_contents(&p.id, 12, "", true)
        .await
        .data
        .expect("read");
    let tip = git(&repo, &["rev-parse", "feat/lien"]).trim().to_string();

    for (verdict, event) in [
        (ReviewState::Commented, "COMMENT"),
        (ReviewState::Approved, "APPROVE"),
        (ReviewState::ChangesRequested, "REQUEST_CHANGES"),
    ] {
        h.core
            .pr_pending_change(&p.id, 12, "", add(2, "Pourquoi 1 ?"))
            .unwrap();
        let seen = h.events.lock().len();
        let done = h
            .core
            .pr_review_submit(
                &p.id,
                12,
                "",
                ReviewAsk {
                    verdict,
                    body: "Deux points.".into(),
                    to_agent: false,
                },
            )
            .await;
        assert!(done.failure.is_none(), "{event}: {:?}", done.failure);
        assert_eq!(done.data.unwrap().lines, 1);
        let sent = reviews_sent(&server);
        let b = sent.last().unwrap();
        assert_eq!(b["event"], event);
        assert_eq!(b["body"], "Deux points.");
        assert_eq!(b["commit_id"], tip.as_str(), "on the commit read");
        assert_eq!(
            (&b["comments"][0]["path"], &b["comments"][0]["line"]),
            (&json!("lien.ts"), &json!(2))
        );
        assert!(
            h.core.pr_pending(&p.id, 12, "").review.notes.is_empty(),
            "published: gone from here"
        );
        assert!(
            h.events.lock()[seen..]
                .iter()
                .any(|e| e["type"] == "prChanged" && e["number"] == 12),
            "the list, the card and the view read it again"
        );
    }
    assert_eq!(
        reviews_sent(&server).len(),
        3,
        "one review each, no comment apart"
    );

    // Nothing to say, or a text missing to ask for changes: refused before GitHub is asked.
    let refused = h
        .core
        .pr_review_submit(
            &p.id,
            12,
            "",
            ReviewAsk {
                verdict: ReviewState::ChangesRequested,
                body: String::new(),
                to_agent: false,
            },
        )
        .await;
    assert!(refused.failure.is_some());
    assert_eq!(reviews_sent(&server).len(), 3);
}

#[tokio::test]
async fn a_refused_review_keeps_it_all_waiting_and_says_why() {
    let h = harness("myreview-refused");
    let server = FakeServer::start().await;
    let (p, repo) = pull_request(
        &h,
        &server,
        "export const ttl = 0;\nexport const lien = 1;\n",
    )
    .await;
    h.core
        .pr_contents(&p.id, 12, "", true)
        .await
        .data
        .expect("read");
    h.core
        .pr_pending_change(&p.id, 12, "", add(2, "Pourquoi 1 ?"))
        .unwrap();
    let ask = || ReviewAsk {
        verdict: ReviewState::Commented,
        body: "Mon texte.".into(),
        to_agent: false,
    };

    // The token without the right to write.
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/reviews",
        403,
        json!({ "message": "Resource not accessible by personal access token" }),
    );
    let r = h.core.pr_review_submit(&p.id, 12, "", ask()).await;
    assert_eq!(r.failure.map(|f| f.kind), Some(FailureKind::Forbidden));
    let kept = h.core.pr_pending(&p.id, 12, "").review;
    assert_eq!(
        (kept.body.as_str(), kept.notes.len()),
        ("Mon texte.", 1),
        "all kept"
    );

    // A commit took the line away meanwhile: GitHub's 422, the diff read again says which remark.
    push_head(&repo, "export const ttl = 0;\n", false);
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/reviews",
        422,
        json!({ "message": "Unprocessable Entity", "errors": ["Line could not be resolved"] }),
    );
    let r = h.core.pr_review_submit(&p.id, 12, "", ask()).await;
    let f = r.failure.expect("refused");
    assert_eq!(f.kind, FailureKind::Lines);
    assert!(
        f.message.contains("Line could not be resolved"),
        "{}",
        f.message
    );
    let kept = h.core.pr_pending(&p.id, 12, "").review;
    assert!(kept.notes[0].stale, "said stale under its line");

    // Published again: the stale one goes in the text, with its place.
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/reviews",
        200,
        json!({ "id": 6 }),
    );
    let r = h.core.pr_review_submit(&p.id, 12, "", ask()).await;
    assert_eq!(r.data.map(|d| (d.lines, d.stale)), Some((0, 1)));
    let b = reviews_sent(&server).last().unwrap().clone();
    assert_eq!(b["body"], "Mon texte.\n\n`lien.ts:2` : Pourquoi 1 ?");
    assert_eq!(b["comments"], json!([]));
}

#[tokio::test]
async fn on_the_users_own_pull_request_only_a_comment_goes() {
    let h = harness("myreview-own");
    let server = FakeServer::start().await;
    let (p, _repo) = pull_request(&h, &server, "export const ttl = 0;\n").await;
    // Opened by the account GitHub is read with (`ada`).
    let mut mine = crate::prread_tests::node(1);
    mine["author"] = json!({ "login": "ada" });
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        json!({ "data": { "repository": { "p12": mine.clone() } } }),
    );
    server.on(
        "POST",
        "/graphql#OpenPulls",
        200,
        json!({ "data": { "repository": { "pullRequests": { "nodes": [mine] } } } }),
    );
    h.core
        .project_prs(&p.id)
        .await
        .data
        .expect("the list says who reads it");
    h.core
        .pr_contents(&p.id, 12, "", true)
        .await
        .data
        .expect("read");
    assert!(own_of(&h, &p.id));
    let ask = |verdict| ReviewAsk {
        verdict,
        body: "Note pour moi.".into(),
        to_agent: false,
    };
    for v in [ReviewState::Approved, ReviewState::ChangesRequested] {
        let r = h.core.pr_review_submit(&p.id, 12, "", ask(v)).await;
        assert!(r.failure.is_some(), "{v:?}");
    }
    assert!(reviews_sent(&server).is_empty(), "nothing asked of GitHub");
    let r = h
        .core
        .pr_review_submit(&p.id, 12, "", ask(ReviewState::Commented))
        .await;
    assert!(r.failure.is_none(), "{:?}", r.failure);
    assert_eq!(reviews_sent(&server)[0]["event"], "COMMENT");
}
