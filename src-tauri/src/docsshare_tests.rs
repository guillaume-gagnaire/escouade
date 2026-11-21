//! Sharing the docs (ESC-257), on temporary git repositories: what is left to share counted from
//! the docs alone, « Partager » that fetches first, commits only the docs and pushes, the remote
//! up to date, ahead or in conflict, a fixed branch and the current one, and the conflict handed
//! to the writer agent (« L'agent résout » / « Annuler ») instead of shown as lines of git.

use crate::core_tests::{git, harness, repo};
use crate::docsshare::*;
use crate::paths::test_dir;
use std::path::{Path, PathBuf};

const PAGE: &str = ".escouade/spec/paiement/echeances.md";
const OTHER_PAGE: &str = ".escouade/spec/paiement/recu.md";
const DECISION: &str = ".escouade/decisions/0001-trois-echeances.md";

fn put(dir: &Path, rel: &str, text: &str) {
    let at = dir.join(rel);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(at, text).unwrap();
}

fn config(dir: &Path) {
    git(dir, &["config", "user.email", "t@t"]);
    git(dir, &["config", "user.name", "t"]);
    git(dir, &["config", "core.autocrlf", "false"]);
}

fn s(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

/// A repository whose `origin` is a bare one, and a clone of it that is a teammate's:
/// (repository, teammate, remote).
fn team(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = test_dir(name);
    let mine = repo(&dir);
    let remote = dir.join("remote.git");
    git(&dir, &["init", "-q", "--bare", "-b", "main", "remote.git"]);
    git(&mine, &["remote", "add", "origin", &s(&remote)]);
    git(&mine, &["push", "-q", "-u", "origin", "main"]);
    git(
        &dir,
        &[
            "clone",
            "-q",
            "-c",
            "core.autocrlf=false",
            &s(&remote),
            "other",
        ],
    );
    let other = dir.join("other");
    config(&other);
    (mine, other, remote)
}

/// The teammate commits `rel` with `text` and pushes it.
fn teammate_pushes(other: &Path, rel: &str, text: &str) {
    put(other, rel, text);
    git(other, &["add", "-A"]);
    git(other, &["commit", "-qm", "les autres"]);
    git(other, &["push", "-q"]);
}

#[tokio::test]
async fn what_is_left_to_share_counts_the_docs_and_nothing_else() {
    let (mine, _, _) = team("share-count");
    put(&mine, PAGE, "Trois échéances.\n");
    put(&mine, DECISION, "Décision.\n");
    put(&mine, "src/code.ts", "const b = 2;\n");
    put(&mine, ".escouade/tickets/x.yml", "title: x\n");
    std::fs::write(mine.join("src/app.ts"), "const a = 2;\n").unwrap();
    let files = pending_files(&s(&mine)).await.unwrap();
    assert_eq!(files, vec![DECISION.to_string(), PAGE.to_string()]);
    // Committed and not pushed: still to share.
    git(
        &mine,
        &["add", "-f", ".escouade/spec", ".escouade/decisions"],
    );
    git(&mine, &["commit", "-qm", "docs"]);
    assert_eq!(pending_files(&s(&mine)).await.unwrap().len(), 2);
}

#[tokio::test]
async fn sharing_commits_only_the_docs_and_pushes_them() {
    let (mine, _, remote) = team("share-push");
    put(&mine, PAGE, "Trois échéances.\n");
    put(&mine, DECISION, "Décision.\n");
    put(&mine, "src/code.ts", "const b = 2;\n");
    std::fs::write(mine.join("src/app.ts"), "const a = 2;\n").unwrap();
    let done = share(&s(&mine), None, false).await.unwrap();
    let Shared::Done {
        committed,
        message,
        pulled,
        received,
    } = done
    else {
        panic!("{done:?}");
    };
    assert_eq!((committed, pulled, received), (2, false, 0));
    assert!(message.starts_with("Docs"), "{message}");
    // The remote has it, with a message that says what it is.
    assert_eq!(git(&remote, &["log", "-1", "--format=%s", "main"]), message);
    let in_commit = git(&mine, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(in_commit.contains("echeances.md") && in_commit.contains("0001-"));
    assert!(!in_commit.contains("code.ts") && !in_commit.contains("app.ts"));
    // The code is as it was: untracked and modified, not committed.
    let status = git(&mine, &["status", "--porcelain"]);
    assert!(
        status.contains("src/code.ts") && status.contains("src/app.ts"),
        "{status}"
    );
    assert!(pending_files(&s(&mine)).await.unwrap().is_empty());
    // Again: nothing.
    assert_eq!(
        share(&s(&mine), None, false).await.unwrap(),
        Shared::Nothing
    );
}

#[tokio::test]
async fn a_message_given_is_the_commits() {
    let (mine, _, remote) = team("share-message");
    put(&mine, PAGE, "x\n");
    share(&s(&mine), Some("  Les échéances  "), false)
        .await
        .unwrap();
    assert_eq!(
        git(&remote, &["log", "-1", "--format=%s", "main"]),
        "Les échéances"
    );
}

#[tokio::test]
async fn a_commit_not_pushed_is_pushed() {
    let (mine, _, remote) = team("share-ahead");
    put(&mine, PAGE, "x\n");
    git(&mine, &["add", "-A"]);
    git(&mine, &["commit", "-qm", "docs à la main"]);
    assert_eq!(
        pending_files(&s(&mine)).await.unwrap(),
        vec![PAGE.to_string()]
    );
    let done = share(&s(&mine), None, false).await.unwrap();
    assert!(
        matches!(done, Shared::Done { committed: 0, .. }),
        "{done:?}"
    );
    assert_eq!(
        git(&remote, &["rev-parse", "main"]),
        git(&mine, &["rev-parse", "HEAD"])
    );
    assert!(pending_files(&s(&mine)).await.unwrap().is_empty());
}

#[tokio::test]
async fn what_the_others_shared_is_fetched_first() {
    let (mine, other, remote) = team("share-behind");
    teammate_pushes(&other, OTHER_PAGE, "Le reçu part par e-mail.\n");
    put(&mine, PAGE, "Trois échéances.\n");
    let done = share(&s(&mine), None, false).await.unwrap();
    let Shared::Done {
        committed,
        pulled,
        received,
        ..
    } = done
    else {
        panic!("{done:?}");
    };
    // Mine shared, the teammate's page received.
    assert_eq!((committed, pulled, received), (1, true, 1));
    // Both are in the repository and on the remote.
    assert!(mine.join(OTHER_PAGE).is_file() && mine.join(PAGE).is_file());
    assert_eq!(
        git(&remote, &["rev-parse", "main"]),
        git(&mine, &["rev-parse", "HEAD"])
    );
    // Others' work alone comes in with nothing to push.
    git(&other, &["pull", "-q", "--no-rebase"]);
    teammate_pushes(&other, ".escouade/spec/autre.md", "Autre.\n");
    let again = share(&s(&mine), None, false).await.unwrap();
    assert!(
        matches!(
            again,
            Shared::Done {
                committed: 0,
                pulled: true,
                ..
            }
        ),
        "{again:?}"
    );
}

#[tokio::test]
async fn a_conflict_is_undone_and_returned_not_shown() {
    let (mine, other, remote) = team("share-conflict");
    teammate_pushes(&other, PAGE, "Deux échéances.\n");
    put(&mine, PAGE, "Trois échéances.\n");
    let before = git(&remote, &["rev-parse", "main"]);
    let out = share(&s(&mine), None, false).await.unwrap();
    let Shared::Conflict { conflict } = out else {
        panic!("{out:?}");
    };
    assert_eq!(conflict.files, vec![PAGE.to_string()]);
    assert_eq!(conflict.target, "origin/main");
    assert_eq!(conflict.agent_id, None);
    // Undone: no merge under way, no marker in the file, the remote untouched, my version kept.
    assert_eq!(crate::git::operation_in_progress(&s(&mine)).await, None);
    assert_eq!(
        std::fs::read_to_string(mine.join(PAGE)).unwrap(),
        "Trois échéances.\n"
    );
    assert_eq!(git(&remote, &["rev-parse", "main"]), before);
    // Still to share.
    assert_eq!(
        pending_files(&s(&mine)).await.unwrap(),
        vec![PAGE.to_string()]
    );
}

#[tokio::test]
async fn a_merge_not_finished_stops_a_new_sharing() {
    let (mine, other, _) = team("share-unfinished");
    teammate_pushes(&other, PAGE, "Deux.\n");
    put(&mine, PAGE, "Trois.\n");
    git(&mine, &["add", "-A"]);
    git(&mine, &["commit", "-qm", "docs"]);
    git(&mine, &["fetch", "-q"]);
    let _ = std::process::Command::new("git")
        .arg("-C")
        .arg(&mine)
        .args(["merge", "--no-edit", "origin/main"])
        .output();
    let e = share(&s(&mine), None, false).await.unwrap_err().to_string();
    assert!(e.contains("rédacteur") || e.contains("writer"), "{e}");
}

#[tokio::test]
async fn a_repository_without_remote_says_so_plainly() {
    let dir = test_dir("share-no-remote");
    let mine = repo(&dir);
    put(&mine, PAGE, "x\n");
    let e = share(&s(&mine), None, false).await.unwrap_err().to_string();
    assert!(
        e.contains("pas de dépôt distant") || e.contains("no remote repository"),
        "{e}"
    );
    assert!(!e.contains("fatal"), "{e}");
    // Nothing committed.
    assert_eq!(git(&mine, &["log", "--oneline"]).lines().count(), 1);
}

#[test]
fn a_refused_credential_is_told_in_a_simple_sentence() {
    use crate::i18n::Lang;
    let refused = anyhow::anyhow!("fatal: Authentication failed for 'https://github.com/a/b.git/'");
    let said = plain(Lang::Fr, "Le partage", &refused).to_string();
    assert!(
        said.contains("s’identifier") && !said.contains("fatal"),
        "{said}"
    );
    let said = plain(Lang::En, "Sharing", &refused).to_string();
    assert!(
        said.contains("sign in") && !said.contains("fatal"),
        "{said}"
    );
    let down = anyhow::anyhow!("fatal: unable to access 'https://x/': Could not resolve host: x");
    assert!(plain(Lang::Fr, "Le partage", &down)
        .to_string()
        .contains("ne répond pas"));
    // Anything else: said with what failed.
    let other = anyhow::anyhow!("boom");
    let said = plain(Lang::Fr, "Le partage", &other).to_string();
    assert!(said.starts_with("Le partage n’a pas abouti"), "{said}");
}

#[test]
fn the_message_tells_what_the_changes_are() {
    use crate::i18n::Lang;
    let files: Vec<String> = [
        ".escouade/spec/a/b.md",
        ".escouade/spec/a/c.md",
        ".escouade/decisions/0001-x.md",
        ".escouade/documents/cahier.pdf",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(
        commit_message(Lang::Fr, &files),
        "Docs : 2 pages de la spec, 1 décision, 1 document"
    );
    assert_eq!(commit_message(Lang::En, &files[..1]), "Docs: 1 spec page");
    assert!(is_docs_path(".escouade/documents/a/b.pdf"));
    assert!(!is_docs_path(".escouade/tickets/a.yml"));
    assert!(!is_docs_path(".escouade/specs/a.md"));
    assert!(!is_docs_path("src/.escouade/spec/a.md"));
}

// ---------- through the core ----------

fn shares(h: &crate::core_tests::Harness) -> Vec<serde_json::Value> {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "docsShare")
        .cloned()
        .collect()
}

/// The harness's project with a remote and a teammate: (project, folder, teammate, remote).
async fn project_with_team(
    h: &crate::core_tests::Harness,
) -> (crate::model::Project, PathBuf, PathBuf, PathBuf) {
    let (p, folder) = h.project(false).await;
    let remote = h.dir.join("remote.git");
    git(
        &h.dir,
        &["init", "-q", "--bare", "-b", "main", "remote.git"],
    );
    git(&folder, &["remote", "add", "origin", &s(&remote)]);
    git(&folder, &["push", "-q", "-u", "origin", "main"]);
    git(
        &h.dir,
        &[
            "clone",
            "-q",
            "-c",
            "core.autocrlf=false",
            &s(&remote),
            "other",
        ],
    );
    let other = h.dir.join("other");
    config(&other);
    (p, folder, other, remote)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_window_is_told_when_the_count_changes() {
    let h = harness("share-event");
    let (p, folder, _, remote) = project_with_team(&h).await;
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    space
        .spec_write("paiement/echeances", |pg| pg.body = "Trois.".into())
        .unwrap();
    h.core.docs_changed(&p.id).await;
    // Told once: the same count says nothing more.
    h.core.docs_changed(&p.id).await;
    let told = shares(&h);
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0]["projectId"], p.id);
    assert_eq!(told[0]["pending"], 1);
    assert!(told[0]["conflict"].is_null());
    let pending = h.core.docs_pending(&p.id).await.unwrap();
    assert_eq!((pending.count, pending.files.len()), (1, 1));
    // Shared: back to none, told.
    h.core.docs_share(&p.id, None, false).await.unwrap();
    let told = shares(&h);
    assert_eq!(told.last().unwrap()["pending"], 0);
    assert!(folder
        .join(".escouade/spec/paiement/echeances.md")
        .is_file());
    assert!(!git(&remote, &["log", "-1", "--format=%s", "main"]).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_conflict_goes_to_the_writer_agent_who_resolves_and_the_user_shares_again() {
    let h = harness("share-agent");
    let (p, folder, other, remote) = project_with_team(&h).await;
    teammate_pushes(&other, PAGE, "Deux échéances.\n");
    put(&folder, PAGE, "Trois échéances.\n");
    let out = h.core.docs_share(&p.id, None, false).await.unwrap();
    assert!(matches!(out, Shared::Conflict { .. }), "{out:?}");
    // Kept, and told to the window.
    let pending = h.core.docs_pending(&p.id).await.unwrap();
    assert_eq!(
        pending.conflict.as_ref().unwrap().files,
        vec![PAGE.to_string()]
    );
    assert_eq!(shares(&h).last().unwrap()["conflict"]["files"][0], PAGE);
    // Sharing again is refused with a sentence, not git's.
    let e = h
        .core
        .docs_share(&p.id, None, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(!e.contains("fatal") && !e.contains("<<<<"), "{e}");
    // « L'agent résout »: a writer agent in the docs' folder, the merge left for it.
    let agents_before = h.core.project_agents(&p.id).len();
    let handed = h.core.docs_share_resolve(&p.id).await.unwrap();
    let agent_id = handed.agent_id.clone().unwrap();
    assert_eq!(h.core.project_agents(&p.id).len(), agents_before + 1);
    assert_eq!(h.agent(&agent_id).cwd, s(&folder));
    assert_eq!(
        crate::git::operation_in_progress(&s(&folder)).await,
        Some("merge")
    );
    assert_eq!(
        crate::git::unmerged(&s(&folder)).await,
        vec![PAGE.to_string()]
    );
    // Once only.
    assert!(h.core.docs_share_resolve(&p.id).await.is_err());
    // Not shared while the agent works.
    let e = h
        .core
        .docs_share(&p.id, None, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("merge") && (e.contains("rédacteur") || e.contains("writer")),
        "{e}"
    );
    // The agent resolves and commits the merge; the user shares again.
    put(&folder, PAGE, "Deux ou trois échéances.\n");
    git(&folder, &["add", "-A", ".escouade"]);
    git(&folder, &["commit", "-qm", "merge des docs"]);
    let done = h.core.docs_share(&p.id, None, false).await.unwrap();
    assert!(matches!(done, Shared::Done { .. }), "{done:?}");
    assert!(h.core.docs_pending(&p.id).await.unwrap().conflict.is_none());
    assert_eq!(
        git(&remote, &["rev-parse", "main"]),
        git(&folder, &["rev-parse", "HEAD"])
    );
    assert_eq!(
        git(
            &remote,
            &["show", "main:.escouade/spec/paiement/echeances.md"]
        ),
        "Deux ou trois échéances."
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_a_conflict_leaves_the_docs_ready_to_share_again() {
    let h = harness("share-cancel");
    let (p, folder, other, remote) = project_with_team(&h).await;
    teammate_pushes(&other, PAGE, "Deux échéances.\n");
    put(&folder, PAGE, "Trois échéances.\n");
    h.core.docs_share(&p.id, None, false).await.unwrap();
    // « Annuler » at once: the conflict is forgotten.
    h.core.docs_share_cancel(&p.id).await.unwrap();
    assert!(h.core.docs_pending(&p.id).await.unwrap().conflict.is_none());
    assert!(shares(&h).last().unwrap()["conflict"].is_null());
    // After the agent started: the merge under way is undone.
    h.core.docs_share(&p.id, None, false).await.unwrap();
    h.core.docs_share_resolve(&p.id).await.unwrap();
    assert_eq!(
        crate::git::operation_in_progress(&s(&folder)).await,
        Some("merge")
    );
    h.core.docs_share_cancel(&p.id).await.unwrap();
    assert_eq!(crate::git::operation_in_progress(&s(&folder)).await, None);
    assert_eq!(
        std::fs::read_to_string(folder.join(PAGE)).unwrap(),
        "Trois échéances.\n"
    );
    assert!(h.core.docs_pending(&p.id).await.unwrap().conflict.is_none());
    // Nothing was lost or pushed: still to share.
    assert_eq!(h.core.docs_pending(&p.id).await.unwrap().count, 1);
    assert_ne!(
        git(&remote, &["rev-parse", "main"]),
        git(&folder, &["rev-parse", "HEAD"])
    );
    // Resolve with nothing to resolve.
    assert!(h.core.docs_share_resolve(&p.id).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fixed_docs_branch_is_shared_from_its_own_worktree() {
    let h = harness("share-fixed");
    let (p, folder, _, remote) = project_with_team(&h).await;
    let mut project = h.core.project(&p.id).unwrap();
    project.docs_branch = "docs".into();
    h.core.update_project(project).unwrap();
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    assert_ne!(space.root, folder);
    space
        .spec_write("paiement/echeances", |pg| pg.body = "Trois.".into())
        .unwrap();
    // The project's own checkout has nothing to share and is left alone.
    put(&folder, "src/code.ts", "const b = 2;\n");
    let pending = h.core.docs_pending(&p.id).await.unwrap();
    assert_eq!(pending.count, 1);
    h.core.docs_share(&p.id, None, false).await.unwrap();
    // The branch of the docs was published; main was not touched.
    let shown = git(
        &remote,
        &["show", "docs:.escouade/spec/paiement/echeances.md"],
    );
    assert!(shown.contains("Trois."), "{shown}");
    assert_eq!(
        git(&remote, &["rev-parse", "main"]),
        git(&folder, &["rev-parse", "HEAD"])
    );
    assert!(git(&folder, &["status", "--porcelain"]).contains("src/code.ts"));
    assert_eq!(h.core.docs_pending(&p.id).await.unwrap().count, 0);
    // A teammate changes the same branch: fetched first, merged, pushed.
    git(&h.dir.join("other"), &["fetch", "-q"]);
    git(&h.dir.join("other"), &["checkout", "-q", "docs"]);
    teammate_pushes(&h.dir.join("other"), OTHER_PAGE, "Reçu.\n");
    space
        .spec_write("paiement/plan", |pg| pg.body = "Plan.".into())
        .unwrap();
    let done = h.core.docs_share(&p.id, None, false).await.unwrap();
    assert!(
        matches!(done, Shared::Done { pulled: true, .. }),
        "{done:?}"
    );
    assert!(space.root.join(OTHER_PAGE).is_file());
}

#[tokio::test]
async fn commits_of_code_not_pushed_are_not_published_without_a_confirmation() {
    let (mine, _, remote) = team("share-code");
    // A commit of code, not pushed, and a page of docs.
    put(&mine, "src/code.ts", "const b = 2;\n");
    git(&mine, &["add", "-A"]);
    git(&mine, &["commit", "-qm", "du code"]);
    put(&mine, PAGE, "Trois échéances.\n");
    let before = git(&remote, &["rev-parse", "main"]);
    assert_eq!(code_commits(&s(&mine)).await.unwrap(), 1);
    let e = share(&s(&mine), None, false).await.unwrap_err().to_string();
    assert!(
        (e.contains("1 commits de code") && e.contains("panneau Git"))
            || (e.contains("1 commits of code") && e.contains("Git panel")),
        "{e}"
    );
    // Nothing left: the remote is as it was. The docs are committed, here, and safe.
    assert_eq!(git(&remote, &["rev-parse", "main"]), before);
    assert_eq!(
        pending_files(&s(&mine)).await.unwrap(),
        vec![PAGE.to_string()]
    );
    assert!(git(&mine, &["status", "--porcelain"]).is_empty());
    // Confirmed: everything is published.
    let done = share(&s(&mine), None, true).await.unwrap();
    assert!(
        matches!(done, Shared::Done { committed: 0, .. }),
        "{done:?}"
    );
    assert_eq!(
        git(&remote, &["rev-parse", "main"]),
        git(&mine, &["rev-parse", "HEAD"])
    );
    assert_eq!(code_commits(&s(&mine)).await.unwrap(), 0);
}

#[tokio::test]
async fn a_branch_of_docs_only_is_pushed_without_a_confirmation() {
    let (mine, _, remote) = team("share-docs-only");
    put(&mine, PAGE, "x\n");
    git(&mine, &["add", "-A"]);
    git(&mine, &["commit", "-qm", "docs"]);
    put(&mine, OTHER_PAGE, "y\n");
    assert_eq!(code_commits(&s(&mine)).await.unwrap(), 0);
    share(&s(&mine), None, false).await.unwrap();
    assert_eq!(
        git(&remote, &["rev-parse", "main"]),
        git(&mine, &["rev-parse", "HEAD"])
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_code_commit_put_by_hand_on_the_fixed_docs_branch_counts_too() {
    let h = harness("share-fixed-code");
    let (p, _folder, _, remote) = project_with_team(&h).await;
    let mut project = h.core.project(&p.id).unwrap();
    project.docs_branch = "docs".into();
    h.core.update_project(project).unwrap();
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    space
        .spec_write("paiement/echeances", |pg| pg.body = "Trois.".into())
        .unwrap();
    h.core.docs_share(&p.id, None, false).await.unwrap();
    // Code committed by hand on the branch of the docs.
    put(&space.root, "src/hand.ts", "x\n");
    git(&space.root, &["add", "-A"]);
    git(&space.root, &["commit", "-qm", "à la main"]);
    space
        .spec_write("paiement/plan", |pg| pg.body = "Plan.".into())
        .unwrap();
    let pending = h.core.docs_pending(&p.id).await.unwrap();
    assert_eq!((pending.count, pending.code_commits), (1, 1));
    let before = git(&remote, &["rev-parse", "docs"]);
    assert!(h.core.docs_share(&p.id, None, false).await.is_err());
    assert_eq!(git(&remote, &["rev-parse", "docs"]), before);
    h.core.docs_share(&p.id, None, true).await.unwrap();
    assert_ne!(git(&remote, &["rev-parse", "docs"]), before);
    assert_eq!(h.core.docs_pending(&p.id).await.unwrap().code_commits, 0);
}

// ---------- the synchronisation with nothing to share (ESC-534) ----------

/// The number of parents of HEAD: 1 after a fast-forward, 2 after a merge commit.
fn parents(dir: &Path) -> usize {
    git(dir, &["rev-list", "--parents", "-n", "1", "HEAD"])
        .split_whitespace()
        .count()
        - 1
}

#[tokio::test]
async fn pulling_up_to_date_docs_says_so() {
    let (mine, _, remote) = team("pull-none");
    let before = git(&mine, &["rev-parse", "HEAD"]);
    let out = pull(&s(&mine)).await.unwrap();
    assert_eq!(out, Shared::Nothing);
    assert_eq!(git(&mine, &["rev-parse", "HEAD"]), before);
    assert_eq!(git(&remote, &["rev-parse", "main"]), before);
}

#[tokio::test]
async fn pulling_brings_the_others_docs_in_fast_forward() {
    let (mine, other, remote) = team("pull-ff");
    put(&other, PAGE, "Trois.\n");
    teammate_pushes(&other, OTHER_PAGE, "Reçu.\n");
    let out = pull(&s(&mine)).await.unwrap();
    assert_eq!(
        out,
        Shared::Done {
            committed: 0,
            message: String::new(),
            pulled: true,
            received: 2,
        }
    );
    // In fast-forward: HEAD is the teammate's commit, no merge commit, nothing pushed.
    assert_eq!(
        git(&mine, &["rev-parse", "HEAD"]),
        git(&remote, &["rev-parse", "main"])
    );
    assert_eq!(parents(&mine), 1);
    assert!(mine.join(PAGE).is_file() && mine.join(OTHER_PAGE).is_file());
    assert!(pending_files(&s(&mine)).await.unwrap().is_empty());
}

#[tokio::test]
async fn pulling_counts_only_the_docs_that_came() {
    let (mine, other, _) = team("pull-code");
    put(&other, "src/other.ts", "const c = 3;\n");
    teammate_pushes(&other, OTHER_PAGE, "Reçu.\n");
    let out = pull(&s(&mine)).await.unwrap();
    assert!(
        matches!(
            out,
            Shared::Done {
                received: 1,
                pulled: true,
                ..
            }
        ),
        "{out:?}"
    );
}

#[tokio::test]
async fn pulling_into_a_dirty_folder_keeps_the_changes() {
    let (mine, other, _) = team("pull-dirty");
    std::fs::write(mine.join("src/app.ts"), "const a = 2;\n").unwrap();
    teammate_pushes(&other, OTHER_PAGE, "Reçu.\n");
    let out = pull(&s(&mine)).await.unwrap();
    assert!(matches!(out, Shared::Done { received: 1, .. }), "{out:?}");
    assert!(mine.join(OTHER_PAGE).is_file());
    assert_eq!(
        std::fs::read_to_string(mine.join("src/app.ts")).unwrap(),
        "const a = 2;\n"
    );
    // A change of the code the others' work touches too: said plainly, nothing merged half way.
    std::fs::write(mine.join("src/app.ts"), "const a = 3;\n").unwrap();
    teammate_pushes(&other, "src/app.ts", "const a = 4;\n");
    let e = pull(&s(&mine)).await.unwrap_err().to_string();
    assert!(e.contains("gênent") || e.contains("in the way"), "{e}");
    assert_eq!(crate::git::operation_in_progress(&s(&mine)).await, None);
    assert_eq!(
        std::fs::read_to_string(mine.join("src/app.ts")).unwrap(),
        "const a = 3;\n"
    );
}

#[tokio::test]
async fn pulling_a_diverged_branch_merges_and_pushes_nothing() {
    let (mine, other, remote) = team("pull-diverged");
    // A commit of code of mine, not pushed: it stays mine.
    put(&mine, "src/code.ts", "const b = 2;\n");
    git(&mine, &["add", "-A"]);
    git(&mine, &["commit", "-qm", "du code"]);
    teammate_pushes(&other, OTHER_PAGE, "Reçu.\n");
    let theirs = git(&remote, &["rev-parse", "main"]);
    let out = pull(&s(&mine)).await.unwrap();
    assert!(matches!(out, Shared::Done { received: 1, .. }), "{out:?}");
    assert_eq!(parents(&mine), 2);
    assert!(mine.join(OTHER_PAGE).is_file() && mine.join("src/code.ts").is_file());
    assert_eq!(git(&remote, &["rev-parse", "main"]), theirs);
}

#[tokio::test]
async fn pulling_a_conflict_undoes_it_and_returns_it() {
    let (mine, other, _) = team("pull-conflict");
    put(&mine, "src/app.ts", "const a = 2;\n");
    git(&mine, &["add", "-A"]);
    git(&mine, &["commit", "-qm", "mien"]);
    teammate_pushes(&other, "src/app.ts", "const a = 3;\n");
    let out = pull(&s(&mine)).await.unwrap();
    let Shared::Conflict { conflict } = out else {
        panic!("{out:?}");
    };
    assert_eq!(conflict.files, vec!["src/app.ts".to_string()]);
    assert_eq!(conflict.target, "origin/main");
    assert_eq!(crate::git::operation_in_progress(&s(&mine)).await, None);
    assert_eq!(
        std::fs::read_to_string(mine.join("src/app.ts")).unwrap(),
        "const a = 2;\n"
    );
}

#[tokio::test]
async fn pulling_without_remote_or_branch_says_so_plainly() {
    let dir = test_dir("pull-no-remote");
    let mine = repo(&dir);
    let e = pull(&s(&mine)).await.unwrap_err().to_string();
    assert!(
        e.contains("pas de dépôt distant") || e.contains("no remote repository"),
        "{e}"
    );
    assert!(!e.contains("fatal"), "{e}");
    git(&mine, &["checkout", "-q", "--detach"]);
    let e = pull(&s(&mine)).await.unwrap_err().to_string();
    assert!(
        e.contains("aucune branche") || e.contains("any branch"),
        "{e}"
    );
}

#[tokio::test]
async fn pulling_a_branch_not_published_has_nothing_to_bring() {
    let (mine, other, _) = team("pull-unpublished");
    teammate_pushes(&other, OTHER_PAGE, "Reçu.\n");
    git(&mine, &["checkout", "-q", "-b", "docs-a-moi"]);
    assert_eq!(pull(&s(&mine)).await.unwrap(), Shared::Nothing);
    assert!(!mine.join(OTHER_PAGE).is_file());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_core_pulls_and_keeps_a_conflict_for_its_gestures() {
    let h = harness("pull-core");
    let (p, folder, other, _) = project_with_team(&h).await;
    teammate_pushes(&other, OTHER_PAGE, "Reçu.\n");
    let out = h.core.docs_pull(&p.id).await.unwrap();
    assert!(matches!(out, Shared::Done { received: 1, .. }), "{out:?}");
    assert!(folder.join(OTHER_PAGE).is_file());
    // A conflict is kept, as « Partager » keeps it, and a new synchronisation waits for its gestures.
    put(&folder, "src/app.ts", "const a = 2;\n");
    git(&folder, &["add", "-A"]);
    git(&folder, &["commit", "-qm", "mien"]);
    teammate_pushes(&other, "src/app.ts", "const a = 3;\n");
    let out = h.core.docs_pull(&p.id).await.unwrap();
    assert!(matches!(out, Shared::Conflict { .. }), "{out:?}");
    let pending = h.core.docs_pending(&p.id).await.unwrap();
    assert_eq!(
        pending.conflict.map(|c| c.files),
        Some(vec!["src/app.ts".to_string()])
    );
    let e = h.core.docs_pull(&p.id).await.unwrap_err().to_string();
    assert!(e.contains("conflit") || e.contains("conflict"), "{e}");
    h.core.docs_share_cancel(&p.id).await.unwrap();
    assert!(h.core.docs_pending(&p.id).await.unwrap().conflict.is_none());
}
