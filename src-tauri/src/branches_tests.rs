//! Integration tests of the branches of a project's folder: real git repositories, agents of the
//! fake `claude` CLI.

use crate::core::{AgentOptions, BranchRefusal, Integration};
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
async fn the_changes_put_aside_come_back_from_their_own_stash_whatever_was_stashed_since() {
    let h = harness("g1f-stash-by-hash");
    let (_, r) = h.project(false).await;
    let root = r.to_string_lossy().to_string();
    std::fs::write(r.join("src").join("app.ts"), "const a = 99; // wip\n").unwrap();
    let mine = h
        .core
        .put_aside(&root, "side", true)
        .await
        .unwrap()
        .expect("put aside");
    assert_eq!(app_ts(&r), "const a = 1;\n");
    // An agent of the folder stashes its own work on top, before the switch has failed.
    std::fs::write(r.join("src").join("app.ts"), "const a = 5; // agent\n").unwrap();
    git(&r, &["stash", "push", "-q", "-m", "agent"]);
    h.core.take_back(&root, &mine).await.unwrap();
    // Theirs is the one left, untouched; mine is back.
    assert_eq!(app_ts(&r), "const a = 99; // wip\n");
    assert_eq!(
        git(&r, &["stash", "list", "--format=%gs"]),
        "On main: agent"
    );
    assert!(git(&r, &["stash", "show", "-p"]).contains("+const a = 5; // agent"));
}

#[tokio::test]
async fn changes_that_cannot_come_back_are_found_in_a_stash_the_error_names() {
    let h = harness("g1f-stash-kept");
    let (_, r) = h.project(false).await;
    let root = r.to_string_lossy().to_string();
    std::fs::write(r.join("src").join("app.ts"), "const a = 99; // wip\n").unwrap();
    let mine = h
        .core
        .put_aside(&root, "side", true)
        .await
        .unwrap()
        .expect("put aside");
    // The folder was written to meanwhile: git will not put the changes over that.
    std::fs::write(r.join("src").join("app.ts"), "const a = 7; // since\n").unwrap();
    let e = h
        .core
        .take_back(&root, &mine)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains(&mine.message) && e.contains(&mine.hash), "{e}");
    assert!(e.contains(&format!("git stash apply {}", mine.hash)), "{e}");
    // Nothing is lost: the stash is still there, and the folder as it was.
    assert_eq!(
        git(&r, &["stash", "list", "--format=%H"]),
        mine.hash.as_str()
    );
    assert_eq!(app_ts(&r), "const a = 7; // since\n");
    // Dropped by someone meanwhile: told the same way.
    git(&r, &["checkout", "--", "src/app.ts"]);
    git(&r, &["stash", "drop", "-q"]);
    let e = h
        .core
        .take_back(&root, &mine)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains(&mine.message) && e.contains(&mine.hash), "{e}");
}

#[tokio::test]
async fn a_merge_under_way_stops_a_switch_before_anything_is_put_aside() {
    let h = harness("g1f-switch-merging");
    let (p, r) = h.project(false).await;
    work_on(&r, "side", "const a = 2;\n");
    git(&r, &["branch", "other"]);
    // A merge that waits for its commit, and a change of the user's on top.
    git(&r, &["merge", "-q", "--no-commit", "--no-ff", "side"]);
    std::fs::write(r.join("src").join("app.ts"), "const a = 3; // wip\n").unwrap();
    for stash in [false, true] {
        let e = h
            .core
            .branch_switch(&p.id, "other", stash)
            .await
            .unwrap_err();
        assert!(e.downcast_ref::<BranchRefusal>().is_none(), "{e:#}");
        assert!(e.to_string().contains("merge"), "{e}");
    }
    let e = h
        .core
        .branch_create(&p.id, "next", "other", true, true)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("merge"), "{e}");
    // Nothing moved, nothing stashed, the merge and the change as they were.
    assert_eq!(current(&r), "main");
    assert!(!exists(&r, "next"));
    assert_eq!(git(&r, &["stash", "list"]), "");
    assert_eq!(app_ts(&r), "const a = 3; // wip\n");
    assert!(r.join(".git").join("MERGE_HEAD").exists());
    // Making a branch without going there is no leaving.
    h.core
        .branch_create(&p.id, "kept", "other", false, false)
        .await
        .unwrap();
    assert!(exists(&r, "kept"));
}

#[tokio::test]
async fn commits_only_a_detached_head_has_are_not_left_behind_but_a_branch_made_there_keeps_them() {
    let h = harness("g1f-switch-detached");
    let (p, r) = h.project(false).await;
    git(&r, &["branch", "side"]);
    // Detached on a commit a branch has: free to leave.
    git(&r, &["switch", "-q", "--detach"]);
    h.core.branch_switch(&p.id, "side", false).await.unwrap();
    assert_eq!(current(&r), "side");
    // Two commits of its own, no branch has them.
    git(&r, &["switch", "-q", "--detach"]);
    commit_change(&r, "const a = 2;\n", "one");
    commit_change(&r, "const a = 3;\n", "two");
    let head = git(&r, &["rev-parse", "HEAD"]);
    let e = h
        .core
        .branch_switch(&p.id, "main", false)
        .await
        .unwrap_err();
    assert!(e.downcast_ref::<BranchRefusal>().is_none(), "{e:#}");
    assert!(e.to_string().contains("2 commits"), "{e}");
    // A start elsewhere leaves them too; not the new branch's own start at HEAD.
    let e = h
        .core
        .branch_create(&p.id, "next", "main", true, true)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("2 commits"), "{e}");
    assert!(!exists(&r, "next"));
    assert_eq!(current(&r), "");
    assert_eq!(git(&r, &["rev-parse", "HEAD"]), head);
    // A branch made at HEAD holds them: the way out.
    h.core
        .branch_create(&p.id, "rescue", "", true, false)
        .await
        .unwrap();
    assert_eq!(current(&r), "rescue");
    assert_eq!(git(&r, &["rev-parse", "rescue"]), head);
    h.core.branch_switch(&p.id, "main", false).await.unwrap();
    assert_eq!(current(&r), "main");
}

#[tokio::test]
async fn untracked_files_a_switch_would_overwrite_are_named_and_stay_where_they_are() {
    let h = harness("g1f-switch-untracked");
    let (p, r) = h.project(false).await;
    git(&r, &["switch", "-qc", "side"]);
    std::fs::write(r.join("notes.txt"), "theirs\n").unwrap();
    git(&r, &["add", "notes.txt"]);
    git(&r, &["commit", "-qm", "notes"]);
    git(&r, &["switch", "-q", "main"]);
    std::fs::write(r.join("notes.txt"), "mine\n").unwrap();
    // No tracked change to stash, none refused as DIRTY: git's refusal is what comes, in words.
    let e = h
        .core
        .branch_switch(&p.id, "side", false)
        .await
        .unwrap_err();
    assert!(e.downcast_ref::<BranchRefusal>().is_none(), "{e:#}");
    assert!(e.to_string().contains("notes.txt"), "{e}");
    assert!(e.to_string().contains("écraserait"), "{e}");
    assert_eq!(current(&r), "main");
    assert_eq!(
        std::fs::read_to_string(r.join("notes.txt")).unwrap(),
        "mine\n"
    );
    // A new branch from another one is switched to the same way: refused, and not left made.
    let e = h
        .core
        .branch_create(&p.id, "next", "side", true, true)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("écraserait"), "{e}");
    assert!(!exists(&r, "next"));
    assert_eq!(current(&r), "main");
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
async fn an_agent_at_work_in_the_folder_through_another_project_holds_off_a_switch_too() {
    let h = harness("g1f-switch-agent-shared");
    let (p, r) = h.project(false).await;
    git(&r, &["branch", "side"]);
    // Two projects on one checkout: the agent of the other one works in the same folder.
    let twin = h
        .core
        .create_project(
            &r.to_string_lossy(),
            "twin",
            "oklch(0.72 0.12 48)",
            false,
            None,
        )
        .await
        .unwrap();
    let a = h.core.create_agent(&twin.id, None).await.unwrap();
    h.wait("warm-up", |h| h.alive(&a.meta.id)).await;
    let set = |s: AgentStatus| h.core.agent(&a.meta.id).unwrap().lock().meta.status = s;
    let busy = format!("AGENT_WORKING:{}:{}", a.meta.id, a.meta.name);
    set(AgentStatus::Running);
    for project in [&p.id, &twin.id] {
        let e = h
            .core
            .branch_switch(project, "side", true)
            .await
            .unwrap_err();
        assert_eq!(wire(&e), busy);
        let e = h
            .core
            .branch_create(project, "next", "", true, true)
            .await
            .unwrap_err();
        assert_eq!(wire(&e), busy);
    }
    assert_eq!(current(&r), "main");
    assert!(!exists(&r, "next"));
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
async fn a_new_branch_cannot_hide_a_remote_one_by_its_name() {
    let h = harness("g1f-shadow");
    let (p, r) = h.project(false).await;
    remote(&h, &r);
    let e = h.core.branch_check(&p.id, "origin/feat").await.unwrap_err();
    assert!(e.to_string().contains("origin"), "{e}");
    let e = h
        .core
        .branch_create(&p.id, "origin/feat", "", false, false)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("origin"), "{e}");
    assert!(!exists(&r, "origin/feat"));
    h.core.branch_check(&p.id, "originals/feat").await.unwrap();
    // A revision is not a branch: it is not found, and nothing is deleted through it.
    let e = h
        .core
        .branch_delete(&p.id, "main~0", false, true)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("introuvable"), "{e}");
    assert_eq!(current(&r), "main");
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
async fn the_remote_copy_goes_with_a_branch_only_when_it_is_its_own() {
    let h = harness("g1f-remote-copy");
    let (p, r) = h.project(false).await;
    let bare = remote(&h, &r);
    let on_remote =
        |b: &str| !git(&bare, &["branch", "--list", b, "--format=%(refname:short)"]).is_empty();
    // `feature` tracks origin/main: that is not a copy of it, and main (the folder's branch, so
    // the project's base) must stay.
    git(&r, &["branch", "-q", "feature", "main"]);
    git(
        &r,
        &["branch", "-q", "--set-upstream-to=origin/main", "feature"],
    );
    let note = h
        .core
        .branch_delete(&p.id, "feature", true, true)
        .await
        .unwrap()
        .expect("it tells why the remote branch stays");
    assert!(
        note.contains("origin/main") && note.contains("base"),
        "{note}"
    );
    assert!(!exists(&r, "feature") && on_remote("main"));
    // Any other branch that is called otherwise there is no copy either.
    git(&r, &["push", "-q", "origin", "main:elsewhere"]);
    git(
        &r,
        &["branch", "-q", "--track", "renamed", "origin/elsewhere"],
    );
    let note = h
        .core
        .branch_delete(&p.id, "renamed", true, true)
        .await
        .unwrap()
        .expect("it tells why");
    assert!(
        note.contains("origin/elsewhere") && note.contains("nom"),
        "{note}"
    );
    assert!(!exists(&r, "renamed") && on_remote("elsewhere"));
    // The remote's default branch is never deleted, whatever tracks it.
    git(&r, &["remote", "set-head", "origin", "main"]);
    git(&r, &["branch", "-q", "--track", "dflt", "origin/main"]);
    let note = h
        .core
        .branch_delete(&p.id, "dflt", true, true)
        .await
        .unwrap()
        .expect("it tells why");
    assert!(
        note.contains("origin/main") && note.contains("défaut"),
        "{note}"
    );
    assert!(!exists(&r, "dflt") && on_remote("main"));
    // Two local branches on the same remote copy: the other one still needs it.
    git(&r, &["push", "-q", "origin", "main:shared"]);
    git(&r, &["branch", "-q", "--track", "shared", "origin/shared"]);
    git(&r, &["branch", "-q", "--track", "twin", "origin/shared"]);
    let note = h
        .core
        .branch_delete(&p.id, "shared", true, true)
        .await
        .unwrap()
        .expect("it tells why");
    assert!(note.contains("« twin »"), "{note}");
    assert!(!exists(&r, "shared") && exists(&r, "twin") && on_remote("shared"));
    // The upstream of the project's base: its copy is the project's, not the branch's alone.
    git(&r, &["switch", "-qc", "develop", "main"]);
    git(&r, &["push", "-qu", "origin", "develop"]);
    git(&r, &["switch", "-q", "main"]);
    let board = BoardSettings {
        target: "develop".into(),
        ..p.board.clone()
    };
    h.core.board_set(&p.id, board).unwrap();
    let e = h
        .core
        .branch_delete(&p.id, "origin/develop", false, true)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("origin/develop"), "{e}");
    assert!(on_remote("develop"));
    let note = h
        .core
        .branch_delete(&p.id, "develop", true, true)
        .await
        .unwrap()
        .expect("it tells why");
    assert!(
        note.contains("origin/develop") && note.contains("base"),
        "{note}"
    );
    assert!(!exists(&r, "develop") && on_remote("develop"));
    // The default branch cannot be deleted from the list of remote branches either.
    let e = h
        .core
        .branch_delete(&p.id, "origin/main", false, true)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("origin/main"), "{e}");
    assert!(on_remote("main"));
    // Its own copy (same name, nobody else on it) goes, and there is nothing to tell.
    git(&r, &["branch", "-q", "own", "main"]);
    git(&r, &["push", "-qu", "origin", "own"]);
    let note = h
        .core
        .branch_delete(&p.id, "own", true, true)
        .await
        .unwrap();
    assert_eq!(note, None);
    assert!(!exists(&r, "own") && !on_remote("own"));
    // No copy to delete: nothing to tell either.
    git(&r, &["branch", "-q", "alone", "main"]);
    assert_eq!(
        h.core
            .branch_delete(&p.id, "alone", true, true)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn the_pointer_to_the_remotes_default_branch_is_not_a_branch_to_delete() {
    let h = harness("g2-remote-head");
    let (p, r) = h.project(false).await;
    remote(&h, &r);
    // `origin/HEAD` is a symbolic ref: deleting it as a branch would follow it to origin/main.
    git(&r, &["remote", "set-head", "origin", "main"]);
    for force in [false, true] {
        let e = h
            .core
            .branch_delete(&p.id, "origin/HEAD", false, force)
            .await
            .unwrap_err();
        let said = e.to_string();
        assert!(
            said.contains("origin/HEAD") && said.contains("n’est pas une branche"),
            "{said}"
        );
    }
    // Neither the pointer nor the branch it points to has moved.
    assert_eq!(
        git(&r, &["rev-parse", "--abbrev-ref", "origin/HEAD"]),
        "origin/main"
    );
    assert!(!git(&r, &["branch", "-r", "--list", "origin/main"]).is_empty());
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

// ---------- agents on an existing branch, « Intégrer <base> » ----------

/// Commits `content` into `file` of the checkout at `dir`, whatever branch it is on.
fn commit_file(dir: &Path, file: &str, content: &str) {
    std::fs::write(dir.join(file), content).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", &format!("edit {file}")]);
}

/// A branch `branch` from main with one commit that adds `file`; the folder stays on main.
fn branch_adding(r: &Path, branch: &str, file: &str) {
    git(r, &["switch", "-qc", branch, "main"]);
    commit_file(r, file, &format!("{branch}\n"));
    git(r, &["switch", "-q", "main"]);
}

/// What the fake CLI of the agent working in `dir` last read from the app, once it has read more
/// than `already` messages.
async fn told(h: &Harness, dir: &Path, already: usize) -> String {
    h.wait("the agent was told", |h| {
        h.stdin_messages(dir).len() > already
    })
    .await;
    h.stdin_messages(dir).last().unwrap()["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn worktrees_of(r: &Path) -> usize {
    git(r, &["worktree", "list"]).lines().count()
}

#[tokio::test]
async fn an_agent_works_in_a_worktree_on_an_existing_branch_toward_the_projects_base() {
    let h = harness("g4-agent-on-branch");
    // The project's agents have no worktree of their own: this one asked for it.
    let (p, r) = h.project(false).await;
    branch_adding(&r, "feat/login", "login.txt");
    let a = h
        .core
        .create_agent_on_branch(&p.id, "feat/login", None)
        .await
        .unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    assert_eq!(
        (wt.branch.as_str(), wt.base_branch.as_str(), wt.existing),
        ("feat/login", "main", true)
    );
    assert!(Path::new(&wt.path).ends_with(".claude/worktrees/login"));
    assert_eq!(a.meta.cwd, wt.path);
    assert_eq!(
        std::fs::read_to_string(Path::new(&wt.path).join("login.txt")).unwrap(),
        "feat/login\n"
    );
    // No branch was made, the folder is where it was, the agent is the one the sidebar selects.
    assert_eq!(current(&r), "main");
    assert_eq!(
        git(&r, &["branch", "--format=%(refname:short)"]),
        "feat/login\nmain"
    );
    assert_eq!(h.core.ui.read().selected_agent.get(&p.id), Some(&a.meta.id));
    // The picker's list says whose branch it is.
    let list = h.core.branches(&p.id).await.unwrap();
    let held = list.iter().find(|b| b.name == "feat/login").unwrap();
    assert_eq!(held.agent.as_deref(), Some(a.meta.id.as_str()));
    // Its base is the board's target while that exists, else the branch the folder is on.
    git(&r, &["branch", "release"]);
    h.core
        .board_set(
            &p.id,
            BoardSettings {
                target: "release".into(),
                ..p.board.clone()
            },
        )
        .unwrap();
    branch_adding(&r, "feat/other", "other.txt");
    let b = h
        .core
        .create_agent_on_branch(&p.id, "feat/other", None)
        .await
        .unwrap();
    assert_eq!(b.meta.worktree.unwrap().base_branch, "release");
}

#[tokio::test]
async fn an_agent_on_a_remote_branch_works_on_the_local_branch_that_tracks_it() {
    let h = harness("g4-agent-on-remote");
    let (p, r) = h.project(false).await;
    remote(&h, &r);
    git(&r, &["push", "-q", "origin", "main:feat/r"]);
    assert!(!exists(&r, "feat/r"));
    let a = h
        .core
        .create_agent_on_branch(&p.id, "origin/feat/r", None)
        .await
        .unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    assert_eq!((wt.branch.as_str(), wt.existing), ("feat/r", true));
    assert_eq!(
        git(&r, &["rev-parse", "--abbrev-ref", "feat/r@{upstream}"]),
        "origin/feat/r"
    );
    // The same remote branch for another agent: the local one is held now.
    let e = h
        .core
        .create_agent_on_branch(&p.id, "origin/feat/r", None)
        .await
        .unwrap_err();
    assert_eq!(
        wire(&e),
        format!("IN_WORKTREE:{}:{}", a.meta.id, a.meta.name)
    );
}

#[tokio::test]
async fn an_agent_is_refused_the_branch_of_the_folder_or_of_another_worktree_and_nothing_is_made() {
    let h = harness("g4-agent-branch-refused");
    let (p, r) = h.project(false).await;
    remote(&h, &r);
    branch_adding(&r, "taken", "taken.txt");
    branch_adding(&r, "elsewhere", "elsewhere.txt");
    let holder = h
        .core
        .create_agent_on_branch(&p.id, "taken", None)
        .await
        .unwrap();
    // A worktree of no agent holds this one.
    let other = h.dir.join("other-worktree");
    git(
        &r,
        &[
            "worktree",
            "add",
            "-q",
            &other.to_string_lossy(),
            "elsewhere",
        ],
    );
    let (agents, worktrees) = (h.core.agents.read().len(), worktrees_of(&r));
    // The folder's branch, and the remote branch that its branch tracks.
    for name in ["main", "origin/main"] {
        let e = h
            .core
            .create_agent_on_branch(&p.id, name, None)
            .await
            .unwrap_err();
        assert_eq!(
            e.to_string(),
            "C’est la branche du dossier du projet : un agent sans worktree y travaille déjà, ou change de branche d’abord.",
            "{name}"
        );
    }
    // An agent's worktree: the code the window reads. Any other: in words.
    let e = h
        .core
        .create_agent_on_branch(&p.id, "taken", None)
        .await
        .unwrap_err();
    assert_eq!(
        wire(&e),
        format!("IN_WORKTREE:{}:{}", holder.meta.id, holder.meta.name)
    );
    let e = h
        .core
        .create_agent_on_branch(&p.id, "elsewhere", None)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("est prise par le worktree"), "{e}");
    // Not branches at all.
    for name in ["nowhere", "origin/nowhere", "--detach", "", "main~1"] {
        assert!(
            h.core
                .create_agent_on_branch(&p.id, name, None)
                .await
                .is_err(),
            "{name:?}"
        );
    }
    assert_eq!(
        (h.core.agents.read().len(), worktrees_of(&r)),
        (agents, worktrees)
    );
}

#[tokio::test]
async fn deleting_the_agent_of_an_existing_branch_removes_its_worktree_and_keeps_the_branch() {
    let h = harness("g4-agent-branch-delete");
    let (p, r) = h.project(true).await;
    branch_adding(&r, "feat/keep", "keep.txt");
    let a = h
        .core
        .create_agent_on_branch(&p.id, "feat/keep", None)
        .await
        .unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    // An ordinary agent, for comparison: its branch is its own, and goes with its worktree.
    let own = h.core.create_agent(&p.id, None).await.unwrap();
    let own_wt = own.meta.worktree.clone().unwrap();
    assert!(!own_wt.existing);
    h.core.delete_agent(&own.meta.id, true).await.unwrap();
    assert!(!exists(&r, &own_wt.branch));
    // The existing branch stays, with its commit, and the folder goes.
    h.core.delete_agent(&a.meta.id, true).await.unwrap();
    assert!(!Path::new(&wt.path).exists());
    assert!(exists(&r, "feat/keep"));
    assert_eq!(
        git(&r, &["log", "-1", "--format=%s", "feat/keep"]),
        "edit keep.txt"
    );
    // Free again: a new agent takes it up.
    h.core
        .create_agent_on_branch(&p.id, "feat/keep", None)
        .await
        .unwrap();
}

#[tokio::test]
async fn naming_the_agent_of_an_existing_branch_never_renames_the_branch() {
    let h = harness("g4-agent-branch-named");
    let (p, r) = h.project(false).await;
    branch_adding(&r, "feat/mine", "mine.txt");
    let a = h
        .core
        .create_agent_on_branch(&p.id, "feat/mine", None)
        .await
        .unwrap()
        .meta
        .id;
    h.turn(&a, "Create the hello file").await;
    h.wait("named", |h| h.agent(&a).named).await;
    // An ordinary agent's branch takes its new name (`escouade/<name>`); this one is the user's.
    let wt = h.agent(&a).worktree.unwrap();
    assert_eq!(wt.branch, "feat/mine");
    assert_eq!(
        git(&r, &["branch", "--format=%(refname:short)"]),
        "feat/mine\nmain"
    );
    assert_eq!(
        git(Path::new(&wt.path), &["branch", "--show-current"]),
        "feat/mine"
    );
}

#[test]
fn what_an_agent_on_a_branch_is_told_reads_in_both_languages() {
    use crate::core::{
        folder_branch_refusal, integrate_conflict_message, integrate_dirty,
        integrate_rebase_message,
    };
    use crate::i18n::Lang::{En, Fr};
    assert_eq!(
        folder_branch_refusal(Fr),
        "C’est la branche du dossier du projet : un agent sans worktree y travaille déjà, ou change de branche d’abord."
    );
    assert_eq!(
        folder_branch_refusal(En),
        "That’s the branch of the project’s folder: an agent without a worktree already works on it, or switch to another branch first."
    );
    assert_eq!(
        integrate_dirty(Fr),
        "Commite ou mets de côté les changements de l'agent d'abord."
    );
    assert_eq!(
        integrate_dirty(En),
        "Commit or stash the agent’s changes first."
    );
    let files = ["src/a.ts".to_string(), "src/b.ts".to_string()];
    assert_eq!(
        integrate_conflict_message(Fr, "main", &files),
        "Le merge de main dans ta branche a des conflits sur : src/a.ts, src/b.ts. Résous-les, puis commite le merge."
    );
    assert_eq!(
        integrate_conflict_message(En, "main", &files),
        "The merge of main into your branch has conflicts in: src/a.ts, src/b.ts. Resolve them, then commit the merge."
    );
    assert_eq!(
        integrate_rebase_message(Fr, "main"),
        "Rebase ta branche sur main et résous ses conflits."
    );
    assert_eq!(
        integrate_rebase_message(En, "main"),
        "Rebase your branch onto main and resolve its conflicts."
    );
    // What goes to Claude is written in one language.
    for text in [
        integrate_conflict_message(En, "main", &files),
        integrate_rebase_message(En, "main"),
    ] {
        assert_eq!(crate::i18n::check::french_in(&text), None, "{text}");
    }
}

/// An agent of a project whose agents have worktrees, which committed `content` into src/app.ts
/// while main moved on to `main_content` there (or, `None`, to a file of its own); `rebase`: the
/// project's strategy. Returns (agent id, its worktree, the repository).
async fn agent_behind_main(
    h: &Harness,
    agent_content: &str,
    main_content: Option<&str>,
    rebase: bool,
) -> (String, PathBuf, PathBuf) {
    let (p, r) = h.project(true).await;
    h.core
        .board_set(
            &p.id,
            BoardSettings {
                strategy: if rebase { "rebase" } else { "squash" }.into(),
                ..p.board.clone()
            },
        )
        .unwrap();
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.unwrap().path);
    commit_change(&wt, agent_content, "agent");
    match main_content {
        Some(c) => commit_change(&r, c, "main"),
        None => commit_file(&r, "main.txt", "main\n"),
    }
    (a.meta.id, wt, r)
}

#[tokio::test]
async fn integrating_the_base_merges_it_into_the_agents_branch_or_rebases_it_as_the_project_does() {
    // Squash, the default, is a merge here: there is nothing to squash into the agent's branch.
    let h = harness("g4-integrate-merge");
    let (id, wt, _) = agent_behind_main(&h, "const a = 2;\n", None, false).await;
    assert_eq!(h.core.integrate_base(&id).await.unwrap(), Integration::Done);
    assert!(wt.join("main.txt").exists());
    assert_eq!(git(&wt, &["rev-list", "--merges", "--count", "HEAD"]), "1");
    assert_eq!(app_ts(&wt), "const a = 2;\n");
    // Nothing is asked of the agent: it has not been told anything.
    assert!(h.stdin_messages(&wt).is_empty());
    // It has all of main now.
    let head = git(&wt, &["rev-parse", "HEAD"]);
    assert_eq!(
        h.core.integrate_base(&id).await.unwrap(),
        Integration::UpToDate
    );
    assert_eq!(git(&wt, &["rev-parse", "HEAD"]), head);

    let h = harness("g4-integrate-rebase");
    let (id, wt, _) = agent_behind_main(&h, "const a = 2;\n", None, true).await;
    assert_eq!(h.core.integrate_base(&id).await.unwrap(), Integration::Done);
    assert_eq!(git(&wt, &["rev-list", "--merges", "--count", "HEAD"]), "0");
    assert_eq!(
        git(&wt, &["log", "--format=%s", "-3"]),
        "agent\nedit main.txt\ninit"
    );
}

#[tokio::test]
async fn a_merge_that_conflicts_is_left_in_the_worktree_and_handed_to_the_agent() {
    let h = harness("g4-integrate-conflict-merge");
    let (id, wt, _) = agent_behind_main(&h, "const a = 2;\n", Some("const a = 3;\n"), false).await;
    let done = h.core.integrate_base(&id).await.unwrap();
    assert_eq!(
        done,
        Integration::Conflict {
            files: vec!["src/app.ts".into()],
            rebase: false
        }
    );
    // Left as it stopped, for the agent to resolve.
    assert!(app_ts(&wt).contains("<<<<<<<"));
    assert_eq!(
        crate::git::operation_in_progress(&wt.to_string_lossy()).await,
        Some("merge")
    );
    // The agent is asked to resolve the files, in a message of its own (it has no ticket).
    assert_eq!(
        told(&h, &wt, 0).await,
        "Le merge de main dans ta branche a des conflits sur : src/app.ts. Résous-les, puis commite le merge."
    );
    // Asked again before it is done: refused, the merge is not touched.
    h.wait("its turn over", |h| !h.agent(&id).status.is_active())
        .await;
    let e = h.core.integrate_base(&id).await.unwrap_err();
    assert!(e.to_string().contains("merge"), "{e}");
    assert!(app_ts(&wt).contains("<<<<<<<"));
}

#[tokio::test]
async fn a_rebase_that_conflicts_is_undone_and_the_agent_asked_to_rebase() {
    let h = harness("g4-integrate-conflict-rebase");
    let (id, wt, _) = agent_behind_main(&h, "const a = 2;\n", Some("const a = 3;\n"), true).await;
    let tip = git(&wt, &["rev-parse", "HEAD"]);
    assert_eq!(
        h.core.integrate_base(&id).await.unwrap(),
        Integration::Conflict {
            files: vec!["src/app.ts".into()],
            rebase: true
        }
    );
    // Undone: the branch is where it was and nothing is under way.
    assert_eq!(git(&wt, &["rev-parse", "HEAD"]), tip);
    assert_eq!(app_ts(&wt), "const a = 2;\n");
    assert_eq!(
        crate::git::operation_in_progress(&wt.to_string_lossy()).await,
        None
    );
    assert_eq!(
        told(&h, &wt, 0).await,
        "Rebase ta branche sur main et résous ses conflits."
    );
}

#[tokio::test]
async fn a_ticket_agent_is_handed_the_conflict_as_the_board_hands_it() {
    let h = harness("g4-integrate-conflict-ticket");
    let (id, wt, _) = agent_behind_main(&h, "const a = 2;\n", Some("const a = 3;\n"), false).await;
    h.core.agent(&id).unwrap().lock().meta.ticket_id = Some("t1".into());
    h.core.integrate_base(&id).await.unwrap();
    assert_eq!(
        told(&h, &wt, 0).await,
        crate::board::conflict_message(crate::i18n::Lang::Fr, "main", &["src/app.ts".to_string()])
    );
}

#[tokio::test]
async fn the_base_is_not_integrated_while_the_agent_works_or_has_changes_or_cannot_take_it() {
    let h = harness("g4-integrate-refused");
    let (id, wt, r) = agent_behind_main(&h, "const a = 2;\n", None, false).await;
    let head = git(&wt, &["rev-parse", "HEAD"]);
    // Changes in its worktree, tracked ones (a file nobody tracks does not stop a merge).
    std::fs::write(wt.join("src").join("app.ts"), "const a = 9; // wip\n").unwrap();
    let e = h.core.integrate_base(&id).await.unwrap_err();
    assert_eq!(
        e.to_string(),
        "Commite ou mets de côté les changements de l'agent d'abord."
    );
    git(&wt, &["checkout", "--", "src/app.ts"]);
    // During its turn.
    h.core.rename_agent(&id, "lent").await.unwrap();
    h.core
        .send_message(&id, "slow".into(), vec![])
        .await
        .unwrap();
    h.wait("the turn", |h| h.agent(&id).status == AgentStatus::Running)
        .await;
    let e = h.core.integrate_base(&id).await.unwrap_err();
    assert_eq!(
        e.to_string(),
        "Attends la fin du tour de lent pour intégrer main."
    );
    h.core.interrupt(&id).await.unwrap();
    h.wait("its end", |h| !h.agent(&id).status.is_active())
        .await;
    // Archived: restored first.
    h.core.archive_agent(&id, true).await.unwrap();
    assert!(h.core.integrate_base(&id).await.is_err());
    h.core.archive_agent(&id, false).await.unwrap();
    // An agent without a worktree has no branch to bring it into.
    let pid = h.agent(&id).project_id;
    let plain = h
        .core
        .create_agent_with(
            &pid,
            AgentOptions {
                isolated: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(h.core.integrate_base(&plain.meta.id).await.is_err());
    // Its base is gone.
    git(&r, &["branch", "-m", "main", "trunk"]);
    let e = h.core.integrate_base(&id).await.unwrap_err();
    assert!(e.to_string().contains("« main »"), "{e}");
    assert_eq!(git(&wt, &["rev-parse", "HEAD"]), head);
}
