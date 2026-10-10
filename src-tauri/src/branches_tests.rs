//! Integration tests of the branches of a project's folder: real git repositories, agents of the
//! fake `claude` CLI.

use crate::core::BranchRefusal;
use crate::core_tests::{commit_change, git, harness, Harness};
use crate::model::*;
use std::path::{Path, PathBuf};

/// The refusal's code, as the window gets it.
fn wire(e: &anyhow::Error) -> String {
    e.downcast_ref::<BranchRefusal>()
        .unwrap_or_else(|| panic!("not a typed refusal: {e:#}"))
        .wire()
}

fn app_ts(r: &Path) -> String {
    std::fs::read_to_string(r.join("src").join("app.ts")).unwrap()
}

fn current(r: &Path) -> String {
    git(r, &["branch", "--show-current"])
}

fn exists(r: &Path, branch: &str) -> bool {
    !git(r, &["branch", "--list", branch]).is_empty()
}

/// Gives the project's repository `r` a bare remote, origin, that main tracks.
fn remote(h: &Harness, r: &Path) -> PathBuf {
    let bare = h.dir.join("remote.git");
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
    git(r, &["remote", "add", "origin", &bare.to_string_lossy()]);
    git(r, &["push", "-qu", "origin", "main"]);
    bare
}

/// Commits `content` into src/app.ts on a new branch `branch` from main, and comes back to main.
fn work_on(r: &Path, branch: &str, content: &str) {
    git(r, &["switch", "-qc", branch, "main"]);
    commit_change(r, content, branch);
    git(r, &["switch", "-q", "main"]);
}

#[test]
fn the_refusals_read_in_english_and_reach_the_window_as_codes() {
    use crate::i18n::Lang::{En, Fr};
    let held = BranchRefusal::InWorktree {
        branch: "escouade/refacto".into(),
        agent_id: "a1b2c3".into(),
        agent: "Refacto: auth".into(),
    };
    let working = BranchRefusal::AgentWorking {
        agent_id: "a1b2c3".into(),
        agent: "Refacto: auth".into(),
    };
    let unmerged = |commits| BranchRefusal::Unmerged {
        branch: "feat".into(),
        base: "main".into(),
        commits,
    };
    // The name comes last: it may hold a `:`.
    assert_eq!(
        [
            BranchRefusal::Dirty.wire(),
            held.wire(),
            working.wire(),
            unmerged(3).wire()
        ],
        [
            "DIRTY",
            "IN_WORKTREE:a1b2c3:Refacto: auth",
            "AGENT_WORKING:a1b2c3:Refacto: auth",
            "UNMERGED:3"
        ]
    );
    assert_eq!(
        BranchRefusal::Dirty.text(En),
        "There are uncommitted changes in the project’s folder: commit or stash them before switching branches."
    );
    assert_eq!(
        held.text(En),
        "The branch “escouade/refacto” is used by agent Refacto: auth, in its worktree."
    );
    assert_eq!(
        working.text(En),
        "Agent Refacto: auth is working in the project’s folder: wait for the end of its turn."
    );
    assert_eq!(
        [unmerged(1).text(En), unmerged(2).text(En)],
        [
            "“feat” isn’t merged into “main”: 1 commit is in no other branch.",
            "“feat” isn’t merged into “main”: 2 commits are in no other branch."
        ]
    );
    assert_eq!(
        [unmerged(0).text(Fr), unmerged(2).text(Fr)],
        [
            "« feat » n’est pas mergée dans « main » : 0 commit n’est dans aucune autre branche.",
            "« feat » n’est pas mergée dans « main » : 2 commits ne sont dans aucune autre branche."
        ]
    );
    assert_eq!(
        working.text(Fr),
        "L’agent Refacto: auth travaille dans le dossier du projet : attends la fin de son tour."
    );
    assert_eq!(
        crate::core::stash_message(En, "feat"),
        "escouade: before switching to feat"
    );
}

#[tokio::test]
async fn the_branches_of_a_project_name_the_agent_whose_worktree_holds_one() {
    let h = harness("g1-branches-list");
    let (p, r) = h.project(true).await;
    git(&r, &["branch", "side"]);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    let list = h.core.branches(&p.id).await.unwrap();
    let get = |n: &str| {
        list.iter()
            .find(|b| b.name == n)
            .unwrap_or_else(|| panic!("{n}: {list:?}"))
    };
    assert!(get("main").current);
    assert_eq!(get("main").agent, None);
    assert_eq!(get(&wt.branch).agent.as_deref(), Some(a.meta.id.as_str()));
    assert!(get(&wt.branch).worktree.is_some());
    assert_eq!(get("side").agent, None);
}

#[tokio::test]
async fn switching_branch_needs_a_clean_folder_or_puts_its_changes_aside() {
    let h = harness("g1-switch-dirty");
    let (p, r) = h.project(false).await;
    git(&r, &["branch", "side"]);
    // Nothing uncommitted: it switches.
    assert_eq!(
        h.core.branch_switch(&p.id, "side", false).await.unwrap(),
        None
    );
    assert_eq!(current(&r), "side");
    // Changes: refused, nothing moves.
    std::fs::write(r.join("src").join("app.ts"), "const a = 99; // wip\n").unwrap();
    let e = h
        .core
        .branch_switch(&p.id, "main", false)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), "DIRTY");
    assert_eq!(current(&r), "side");
    assert_eq!(app_ts(&r), "const a = 99; // wip\n");
    // With the stash: put aside under a name that says why, then switched.
    let stashed = h.core.branch_switch(&p.id, "main", true).await.unwrap();
    assert_eq!(
        stashed.as_deref(),
        Some("escouade: avant de passer sur main")
    );
    assert_eq!(current(&r), "main");
    assert_eq!(app_ts(&r), "const a = 1;\n");
    assert_eq!(
        git(&r, &["stash", "list", "--format=%gs"]),
        "On side: escouade: avant de passer sur main"
    );
    assert!(git(&r, &["stash", "show", "-p"]).contains("+const a = 99; // wip"));
    // Already there: nothing to do, nothing to put aside.
    std::fs::write(r.join("src").join("app.ts"), "const a = 7;\n").unwrap();
    assert_eq!(
        h.core.branch_switch(&p.id, "main", true).await.unwrap(),
        None
    );
    assert_eq!(app_ts(&r), "const a = 7;\n");
    // A branch that is not there.
    std::fs::write(r.join("src").join("app.ts"), "const a = 1;\n").unwrap();
    assert!(h.core.branch_switch(&p.id, "nowhere", false).await.is_err());
}

#[tokio::test]
async fn a_switch_that_fails_after_the_stash_puts_the_changes_back() {
    let h = harness("g1-switch-pop");
    let (p, r) = h.project(false).await;
    // `side` tracks notes.txt, which an untracked file of the folder would be overwritten by.
    git(&r, &["switch", "-qc", "side"]);
    std::fs::write(r.join("notes.txt"), "theirs\n").unwrap();
    git(&r, &["add", "notes.txt"]);
    git(&r, &["commit", "-qm", "notes"]);
    git(&r, &["switch", "-q", "main"]);
    std::fs::write(r.join("notes.txt"), "mine\n").unwrap();
    std::fs::write(r.join("src").join("app.ts"), "const a = 99; // wip\n").unwrap();
    git(&r, &["add", "src/app.ts"]);
    assert!(h.core.branch_switch(&p.id, "side", true).await.is_err());
    assert_eq!(current(&r), "main");
    // The changes are back, staged as they were, and no stash is left.
    assert_eq!(app_ts(&r), "const a = 99; // wip\n");
    assert_eq!(
        git(&r, &["status", "--porcelain"]),
        "M  src/app.ts\n?? notes.txt"
    );
    assert_eq!(git(&r, &["stash", "list"]), "");
}

#[tokio::test]
async fn an_agent_at_work_in_the_project_folder_holds_off_a_switch() {
    let h = harness("g1-switch-agent");
    let (p, r) = h.project(false).await;
    git(&r, &["branch", "side"]);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    h.wait("warm-up", |h| h.alive(&a.meta.id)).await;
    let set = |s: AgentStatus| h.core.agent(&a.meta.id).unwrap().lock().meta.status = s;
    let busy = format!("AGENT_WORKING:{}:{}", a.meta.id, a.meta.name);
    set(AgentStatus::Running);
    let e = h.core.branch_switch(&p.id, "side", true).await.unwrap_err();
    assert_eq!(wire(&e), busy);
    // Nor is a new branch switched to; made without switching, it is.
    let e = h
        .core
        .branch_create(&p.id, "next", "", true, false)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), busy);
    assert!(!exists(&r, "next"));
    h.core
        .branch_create(&p.id, "kept", "", false, false)
        .await
        .unwrap();
    assert!(exists(&r, "kept"));
    // Waiting for an answer, its turn is still under way.
    set(AgentStatus::Waiting);
    let e = h
        .core
        .branch_switch(&p.id, "side", false)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), busy);
    assert_eq!(current(&r), "main");
    set(AgentStatus::Idle);
    h.core.branch_switch(&p.id, "side", false).await.unwrap();
    assert_eq!(current(&r), "side");
}

#[tokio::test]
async fn a_branch_an_agents_worktree_holds_is_neither_switched_to_nor_deleted() {
    let h = harness("g1-in-worktree");
    let (p, r) = h.project(true).await;
    git(&r, &["branch", "side"]);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    let held = format!("IN_WORKTREE:{}:{}", a.meta.id, a.meta.name);
    let e = h
        .core
        .branch_switch(&p.id, &wt.branch, true)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), held);
    let e = h
        .core
        .branch_delete(&p.id, &wt.branch, false, true)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), held);
    assert_eq!(current(&r), "main");
    assert!(exists(&r, &wt.branch));
    // An agent at work in its own worktree does not hold the project's folder.
    h.wait("warm-up", |h| h.alive(&a.meta.id)).await;
    h.core.agent(&a.meta.id).unwrap().lock().meta.status = AgentStatus::Running;
    h.core.branch_switch(&p.id, "side", false).await.unwrap();
    assert_eq!(current(&r), "side");
    // The worktree of no agent: said in words.
    let elsewhere = h.dir.join("elsewhere");
    git(
        &r,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "theirs",
            &elsewhere.to_string_lossy(),
        ],
    );
    for e in [
        h.core
            .branch_switch(&p.id, "theirs", false)
            .await
            .unwrap_err(),
        h.core
            .branch_delete(&p.id, "theirs", false, true)
            .await
            .unwrap_err(),
    ] {
        assert!(e.downcast_ref::<BranchRefusal>().is_none(), "{e:#}");
        assert!(e.to_string().contains("« theirs »"), "{e}");
    }
    assert!(exists(&r, "theirs"));
}

#[tokio::test]
async fn a_new_branch_is_made_from_a_start_and_switched_to_when_asked() {
    let h = harness("g1-create");
    let (p, r) = h.project(false).await;
    let first = git(&r, &["rev-parse", "HEAD"]);
    commit_change(&r, "const a = 2;\n", "second");
    // Without switching: made, the folder stays where it is.
    assert_eq!(
        h.core
            .branch_create(&p.id, "old", &first, false, false)
            .await
            .unwrap(),
        None
    );
    assert_eq!(git(&r, &["rev-parse", "old"]), first);
    assert_eq!(current(&r), "main");
    // From HEAD, with changes, switching: they go along, nothing of the folder changes.
    std::fs::write(r.join("src").join("app.ts"), "const a = 9; // wip\n").unwrap();
    assert_eq!(
        h.core
            .branch_create(&p.id, "wip", "", true, false)
            .await
            .unwrap(),
        None
    );
    assert_eq!(current(&r), "wip");
    assert_eq!(app_ts(&r), "const a = 9; // wip\n");
    // From another commit, with changes: refused unless they are put aside; nothing made then.
    let e = h
        .core
        .branch_create(&p.id, "from-old", "old", true, false)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), "DIRTY");
    assert!(!exists(&r, "from-old"));
    let stashed = h
        .core
        .branch_create(&p.id, "from-old", "old", true, true)
        .await
        .unwrap();
    assert_eq!(
        stashed.as_deref(),
        Some("escouade: avant de passer sur from-old")
    );
    assert_eq!(current(&r), "from-old");
    assert_eq!(app_ts(&r), "const a = 1;\n");
    // A name git refuses, one taken, a start that is not one: refused before anything is put aside.
    std::fs::write(r.join("src").join("app.ts"), "const a = 5;\n").unwrap();
    for (name, start) in [("a..b", ""), ("main", ""), ("new", "nowhere")] {
        assert!(
            h.core
                .branch_create(&p.id, name, start, true, true)
                .await
                .is_err(),
            "{name}"
        );
    }
    assert_eq!(git(&r, &["stash", "list"]).lines().count(), 1);
    assert_eq!(app_ts(&r), "const a = 5;\n");
    assert!(!exists(&r, "new"));
}

#[tokio::test]
async fn a_new_branch_name_is_checked_the_way_git_does_and_against_the_branches_there() {
    let h = harness("g1-check");
    let (p, _) = h.project(false).await;
    h.core.branch_check(&p.id, "feat/x").await.unwrap();
    let e = h.core.branch_check(&p.id, "a..b").await.unwrap_err();
    assert!(e.to_string().contains("« a..b »"), "{e}");
    assert_eq!(
        h.core
            .branch_check(&p.id, "main")
            .await
            .unwrap_err()
            .to_string(),
        "La branche « main » existe déjà."
    );
}

#[tokio::test]
async fn a_branch_merged_into_the_base_is_deleted_at_once_another_only_when_forced() {
    let h = harness("g1-delete");
    let (p, r) = h.project(false).await;
    git(&r, &["branch", "done"]);
    work_on(&r, "open", "const a = 2;\n");
    work_on(&r, "squashed", "const a = 3;\n");
    git(&r, &["merge", "-q", "--squash", "squashed"]);
    git(&r, &["commit", "-qm", "squashed"]);

    h.core
        .branch_delete(&p.id, "done", false, false)
        .await
        .unwrap();
    // Squash-merged: its commits are not in main, its work is.
    h.core
        .branch_delete(&p.id, "squashed", false, false)
        .await
        .unwrap();
    let e = h
        .core
        .branch_delete(&p.id, "open", false, false)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), "UNMERGED:1");
    assert!(exists(&r, "open"));
    h.core
        .branch_delete(&p.id, "open", false, true)
        .await
        .unwrap();
    assert!(!exists(&r, "open") && !exists(&r, "done") && !exists(&r, "squashed"));
    // Never the folder's own branch, even forced; nor one that is not there.
    let e = h
        .core
        .branch_delete(&p.id, "main", false, true)
        .await
        .unwrap_err();
    assert!(e.downcast_ref::<BranchRefusal>().is_none(), "{e:#}");
    assert!(e.to_string().contains("« main »"), "{e}");
    assert!(exists(&r, "main"));
    assert!(h
        .core
        .branch_delete(&p.id, "nowhere", false, true)
        .await
        .is_err());
}

#[tokio::test]
async fn merged_means_merged_into_the_boards_target_branch() {
    let h = harness("g1-delete-target");
    let (p, r) = h.project(false).await;
    // feat went into develop, not into main.
    work_on(&r, "feat", "const a = 2;\n");
    git(&r, &["switch", "-qc", "develop", "main"]);
    git(&r, &["merge", "-q", "--no-ff", "-m", "merge feat", "feat"]);
    git(&r, &["switch", "-q", "main"]);
    // Without a target, the base is the folder's branch: develop has feat's commit, main not.
    let e = h
        .core
        .branch_delete(&p.id, "feat", false, false)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), "UNMERGED:0");
    assert!(h.core.branches_merged(&p.id).await.unwrap().is_empty());

    let board = BoardSettings {
        target: "develop".into(),
        ..p.board.clone()
    };
    h.core.board_set(&p.id, board).unwrap();
    // main is in develop too, but it is the folder's branch.
    assert_eq!(h.core.branches_merged(&p.id).await.unwrap(), ["feat"]);
    h.core
        .branch_delete(&p.id, "feat", false, false)
        .await
        .unwrap();
    // The base is never merged "into itself": its own commits count.
    let e = h
        .core
        .branch_delete(&p.id, "develop", false, false)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), "UNMERGED:2");
    assert!(exists(&r, "develop"));
}

#[tokio::test]
async fn the_merged_branches_to_clean_up_leave_out_the_folders_the_bases_and_the_worktrees() {
    let h = harness("g1-merged-list");
    let (p, r) = h.project(true).await;
    // Its branch starts at main: merged, but its worktree holds it.
    h.core.create_agent(&p.id, None).await.unwrap();
    git(&r, &["branch", "old"]);
    work_on(&r, "open", "const a = 2;\n");
    work_on(&r, "squashed", "const a = 3;\n");
    git(&r, &["merge", "-q", "--squash", "squashed"]);
    git(&r, &["commit", "-qm", "squashed"]);
    git(&r, &["switch", "-qc", "here", "main"]);
    let board = BoardSettings {
        target: "main".into(),
        ..p.board.clone()
    };
    h.core.board_set(&p.id, board).unwrap();
    assert_eq!(
        h.core.branches_merged(&p.id).await.unwrap(),
        ["old", "squashed"]
    );
}

#[tokio::test]
async fn a_branch_is_deleted_with_its_remote_one_when_asked() {
    let h = harness("g1-delete-remote");
    let (p, r) = h.project(false).await;
    let bare = remote(&h, &r);
    let on_remote =
        |b: &str| !git(&bare, &["branch", "--list", b, "--format=%(refname:short)"]).is_empty();
    // feat: published, then merged into main; the remote has one more commit of someone else's.
    git(&r, &["switch", "-qc", "feat"]);
    commit_change(&r, "const a = 2;\n", "feat");
    git(&r, &["push", "-qu", "origin", "feat"]);
    git(&r, &["switch", "-qc", "theirs"]);
    commit_change(&r, "const a = 3;\n", "theirs");
    git(&r, &["push", "-q", "origin", "theirs:feat"]);
    git(&r, &["switch", "-q", "main"]);
    git(&r, &["branch", "-qD", "theirs"]);
    git(&r, &["merge", "-q", "--no-ff", "-m", "merge feat", "feat"]);

    // Both would lose the remote's commit.
    let e = h
        .core
        .branch_delete(&p.id, "feat", true, false)
        .await
        .unwrap_err();
    assert_eq!(wire(&e), "UNMERGED:1");
    assert!(exists(&r, "feat") && on_remote("feat"));
    h.core
        .branch_delete(&p.id, "feat", true, true)
        .await
        .unwrap();
    assert!(!exists(&r, "feat") && !on_remote("feat"));
    assert_eq!(git(&r, &["branch", "-r", "--list", "origin/feat"]), "");
    // Without asking for it, the remote branch stays.
    git(&r, &["branch", "kept"]);
    git(&r, &["push", "-qu", "origin", "kept"]);
    h.core
        .branch_delete(&p.id, "kept", false, false)
        .await
        .unwrap();
    assert!(!exists(&r, "kept") && on_remote("kept"));
    // A remote branch alone (the picker lists it as origin/kept): deleted on its remote.
    h.core
        .branch_delete(&p.id, "origin/kept", false, false)
        .await
        .unwrap();
    assert!(!on_remote("kept"));
    assert_eq!(git(&r, &["branch", "-r", "--list", "origin/kept"]), "");
}

#[tokio::test]
async fn two_branches_of_a_project_are_compared() {
    let h = harness("g1-diff-refs");
    let (p, r) = h.project(false).await;
    work_on(&r, "feat", "const a = 2;\n");
    let d = h.core.diff_refs(&p.id, "main", "feat").await.unwrap();
    assert!(
        d.contains("-const a = 1;") && d.contains("+const a = 2;"),
        "{d}"
    );
    assert!(h.core.diff_refs(&p.id, "main", "--output=x").await.is_err());
}
