//! The background fetch (ESC-561): its settings, general and of a project, followed without a
//! restart; its time limit; its state for the Git panel; credentials that never loop; the user's
//! configuration of git, as it is.

use crate::core::SyncOp;
use crate::core_tests::{commit_change, git, harness, Harness};
use crate::model::{
    FetchFailureKind, FetchMode, FetchPlan, GitFetchSettings, GitInfo, Project, ProjectFetch,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The project's repository `r` given a bare remote (origin, tracked by main), and a second
/// clone of it to push commits the project does not have yet.
fn with_remote(dir: &Path, r: &Path) -> (String, PathBuf) {
    let bare = dir.join("remote.git").to_string_lossy().replace('\\', "/");
    git(dir, &["init", "-q", "--bare", "-b", "main", &bare]);
    git(r, &["remote", "add", "origin", &bare]);
    git(r, &["push", "-qu", "origin", "main"]);
    let other = dir.join("other");
    git(dir, &["clone", "-q", &bare, &other.to_string_lossy()]);
    git(&other, &["config", "core.autocrlf", "false"]);
    (bare, other)
}

/// A commit pushed by someone else: the project is one behind once it fetched.
fn push_theirs(other: &Path, n: u32) {
    commit_change(other, &format!("const a = {n};\n"), &format!("theirs {n}"));
    git(other, &["push", "-q"]);
}

async fn info(h: &Harness, p: &Project) -> GitInfo {
    h.core.compute_git(&p.id).await;
    h.core.git_cache.read().get(&p.id).cloned().unwrap()
}

async fn root(h: &Harness, p: &Project) -> String {
    h.core.project_roots_of(&p.id).await.remove(0)
}

/// The project's own fetch, as the window saves it.
fn set_project(h: &Harness, p: &Project, f: ProjectFetch) {
    let mut cur = h.core.project(&p.id).unwrap();
    cur.git_fetch = f;
    h.core.update_project(cur).unwrap();
}

/// Its last background fetch made as old as `ago`.
fn last_try_ago(h: &Harness, root: &str, ago: Duration) {
    h.core.fetches.lock().get_mut(root).unwrap().last_try = Some(Instant::now() - ago);
}

/// A fake ssh for the remote `ssh://example.invalid/x`, through the user's `core.sshCommand`:
/// `body` is the shell it runs; each run is counted in `runs`.
fn fake_ssh(dir: &Path, r: &Path, body: &str) -> PathBuf {
    let runs = dir.join("ssh-runs");
    let script = dir.join("fake-ssh.sh");
    let runs_sh = runs.to_string_lossy().replace('\\', "/");
    std::fs::write(
        &script,
        format!("#!/bin/sh\necho run >> '{runs_sh}'\n{body}\n"),
    )
    .unwrap();
    let script_sh = script.to_string_lossy().replace('\\', "/");
    git(
        r,
        &["config", "core.sshCommand", &format!("sh '{script_sh}'")],
    );
    // Else git runs it once more first (`-G`) to know what ssh it is.
    git(r, &["config", "ssh.variant", "ssh"]);
    git(
        r,
        &[
            "remote",
            "set-url",
            "origin",
            "ssh://git@example.invalid/x.git",
        ],
    );
    runs
}

fn runs(file: &Path) -> usize {
    std::fs::read_to_string(file)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

#[test]
fn a_projects_own_values_win_over_the_general_setting_each_within_its_bounds() {
    let general = GitFetchSettings::default();
    assert_eq!(
        (general.auto, general.every_min, general.timeout_secs),
        (true, 5, 300)
    );
    let off = GitFetchSettings {
        auto: false,
        ..general.clone()
    };
    let own = |mode, every_min, timeout_secs| ProjectFetch {
        mode,
        every_min,
        timeout_secs,
    };
    let plan = |on, every_min, timeout_secs| FetchPlan {
        on,
        every_min,
        timeout_secs,
    };
    // As the general setting: on or off with it, its interval and its limit.
    assert_eq!(
        own(FetchMode::Default, 0, 0).plan(&general),
        plan(true, 5, 300)
    );
    assert_eq!(
        own(FetchMode::Default, 0, 0).plan(&off),
        plan(false, 5, 300)
    );
    // Always and never, whatever the general setting says.
    assert_eq!(own(FetchMode::Always, 0, 0).plan(&off), plan(true, 5, 300));
    assert_eq!(
        own(FetchMode::Never, 0, 0).plan(&general),
        plan(false, 5, 300)
    );
    // Its own interval and limit.
    assert_eq!(
        own(FetchMode::Default, 60, 900).plan(&general),
        plan(true, 60, 900)
    );
    // Out of bounds, brought back to them: 1 min to 24 h, 30 s to 30 min.
    assert_eq!(
        own(FetchMode::Default, 5000, 5).plan(&general),
        plan(true, 1440, 30)
    );
    let wild = GitFetchSettings {
        auto: true,
        every_min: 0,
        timeout_secs: 99_999,
    };
    assert_eq!(
        own(FetchMode::Default, 0, 0).plan(&wild),
        plan(true, 1, 1800)
    );
    assert_eq!(
        wild.clamped(),
        GitFetchSettings {
            auto: true,
            every_min: 1,
            timeout_secs: 1800
        }
    );
    // A project's 0 stays 0: the general setting's.
    assert_eq!(
        own(FetchMode::Always, 0, 10).clamped(),
        own(FetchMode::Always, 0, 30)
    );
    // A repository of two projects: fetched when one wants it, as often as the most eager, as
    // long as the most patient.
    assert_eq!(
        FetchPlan::merge([plan(true, 15, 60), plan(false, 1, 1800), plan(true, 5, 120)]),
        Some(plan(true, 5, 120))
    );
    assert_eq!(FetchPlan::merge([plan(false, 5, 300)]), None);
}

#[tokio::test]
async fn turned_off_no_background_fetch_runs_and_a_projects_always_wins() {
    let h = harness("fetch-off");
    let (p, r) = h.project(false).await;
    let (_, other) = with_remote(&h.dir, &r);
    push_theirs(&other, 2);
    h.set_settings(|s| s.git_fetch.auto = false);
    h.core.fetch_all().await;
    let i = info(&h, &p).await;
    assert_eq!((i.behind, i.last_fetch), (0, None), "nothing fetched");
    assert!(h.core.fetches.lock().is_empty());
    // The project's « Toujours » fetches it all the same, at once: no restart.
    set_project(
        &h,
        &p,
        ProjectFetch {
            mode: FetchMode::Always,
            ..Default::default()
        },
    );
    h.core.fetch_all().await;
    let i = info(&h, &p).await;
    assert_eq!(i.behind, 1);
    assert!(i.last_fetch.is_some());
    // And its « Jamais » none, the general setting on.
    h.set_settings(|s| s.git_fetch.auto = true);
    set_project(
        &h,
        &p,
        ProjectFetch {
            mode: FetchMode::Never,
            ..Default::default()
        },
    );
    push_theirs(&other, 3);
    last_try_ago(&h, &root(&h, &p).await, Duration::from_secs(3600));
    h.core.fetch_all().await;
    assert_eq!(info(&h, &p).await.behind, 1);
}

#[tokio::test]
async fn the_interval_is_followed_and_a_change_holds_without_a_restart() {
    let h = harness("fetch-every");
    let (p, r) = h.project(false).await;
    let (_, other) = with_remote(&h.dir, &r);
    let root = root(&h, &p).await;
    // Never fetched: due at once.
    push_theirs(&other, 2);
    h.core.fetch_all().await;
    assert_eq!(info(&h, &p).await.behind, 1);
    // Fetched just now: not due before its 5 min.
    push_theirs(&other, 3);
    h.core.fetch_all().await;
    assert_eq!(info(&h, &p).await.behind, 1);
    // 6 min later it is due…
    last_try_ago(&h, &root, Duration::from_secs(6 * 60));
    // …unless the interval became 10 min meanwhile, read at this very look.
    h.set_settings(|s| s.git_fetch.every_min = 10);
    h.core.fetch_all().await;
    assert_eq!(info(&h, &p).await.behind, 1);
    // Back to 5 min: it goes.
    h.set_settings(|s| s.git_fetch.every_min = 5);
    h.core.fetch_all().await;
    assert_eq!(info(&h, &p).await.behind, 2);
    // A project's own interval wins: 30 min, so 6 min are not enough.
    push_theirs(&other, 4);
    last_try_ago(&h, &root, Duration::from_secs(6 * 60));
    set_project(
        &h,
        &p,
        ProjectFetch {
            every_min: 30,
            ..Default::default()
        },
    );
    h.core.fetch_all().await;
    assert_eq!(info(&h, &p).await.behind, 2);
    last_try_ago(&h, &root, Duration::from_secs(31 * 60));
    h.core.fetch_all().await;
    assert_eq!(info(&h, &p).await.behind, 3);
}

#[tokio::test]
async fn values_out_of_bounds_are_brought_back_to_them_when_saved() {
    let h = harness("fetch-bounds");
    let (p, _r) = h.project(false).await;
    h.set_settings(|s| {
        s.git_fetch.every_min = 0;
        s.git_fetch.timeout_secs = 5;
    });
    let s = h.core.settings.read().git_fetch.clone();
    assert_eq!((s.every_min, s.timeout_secs), (1, 30));
    h.set_settings(|s| {
        s.git_fetch.every_min = 100_000;
        s.git_fetch.timeout_secs = 100_000;
    });
    let s = h.core.settings.read().git_fetch.clone();
    assert_eq!((s.every_min, s.timeout_secs), (1440, 1800));
    set_project(
        &h,
        &p,
        ProjectFetch {
            mode: FetchMode::Always,
            every_min: 2000,
            timeout_secs: 1,
        },
    );
    let f = h.core.project(&p.id).unwrap().git_fetch;
    assert_eq!((f.every_min, f.timeout_secs), (1440, 30));
}

#[tokio::test]
async fn a_fetch_past_its_time_limit_is_stopped_and_told_as_such() {
    let h = harness("fetch-timeout");
    let (p, r) = h.project(false).await;
    with_remote(&h.dir, &r);
    fake_ssh(&h.dir, &r, "sleep 60");
    let root = root(&h, &p).await;
    let start = Instant::now();
    h.core.fetch_root(&root, 2).await;
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "{:?}",
        start.elapsed()
    );
    let f = info(&h, &p).await.fetch_failure.expect("a failure");
    assert_eq!((f.kind, f.timeout_secs), (FetchFailureKind::Timeout, 2));
}

#[tokio::test]
async fn the_last_failure_and_its_reason_are_kept_until_a_success() {
    let h = harness("fetch-state");
    let (p, r) = h.project(false).await;
    let (bare, other) = with_remote(&h.dir, &r);
    // The remote gone: a failure, with git's words.
    git(
        &r,
        &["remote", "set-url", "origin", &format!("{bare}-gone")],
    );
    h.core.fetch_all().await;
    let f = info(&h, &p).await.fetch_failure.expect("a failure");
    assert_eq!(f.kind, FetchFailureKind::Other);
    assert!(f.message.contains("gone"), "{}", f.message);
    assert!(f.at > 0);
    assert_eq!(f.timeout_secs, 300);
    // Sent to the window with the git state.
    let sent = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "git")
        .cloned()
        .unwrap();
    assert_eq!(sent["git"]["fetchFailure"]["kind"], "other");
    // Back: the next fetch due goes well, and the failure is forgotten.
    git(&r, &["remote", "set-url", "origin", &bare]);
    push_theirs(&other, 2);
    last_try_ago(&h, &root(&h, &p).await, Duration::from_secs(3600));
    h.core.fetch_all().await;
    let i = info(&h, &p).await;
    assert_eq!((i.fetch_failure, i.behind), (None, 1));
}

#[tokio::test]
async fn a_repository_that_asks_for_credentials_says_it_once_and_waits_for_the_users_fetch() {
    let h = harness("fetch-credentials");
    let (p, r) = h.project(false).await;
    let (bare, _) = with_remote(&h.dir, &r);
    let ran = fake_ssh(
        &h.dir,
        &r,
        "echo 'git@example.invalid: Permission denied (publickey).' >&2\nexit 255",
    );
    let root = root(&h, &p).await;
    h.core.fetch_all().await;
    assert_eq!(runs(&ran), 1);
    let f = info(&h, &p).await.fetch_failure.expect("a failure");
    assert_eq!(f.kind, FetchFailureKind::Credentials);
    // Its interval past, again and again: never tried again in the background.
    for _ in 0..3 {
        last_try_ago(&h, &root, Duration::from_secs(3600));
        h.core.fetch_all().await;
    }
    assert_eq!(runs(&ran), 1, "no loop");
    // The user's own fetch goes through (credentials given): the background fetch goes on.
    git(&r, &["config", "--unset", "core.sshCommand"]);
    git(&r, &["remote", "set-url", "origin", &bare]);
    h.core.git_sync(&p.id, SyncOp::Fetch).await.unwrap();
    assert_eq!(info(&h, &p).await.fetch_failure, None);
    let before = h.core.fetches.lock().get(&root).unwrap().last_try;
    last_try_ago(&h, &root, Duration::from_secs(3600));
    h.core.fetch_all().await;
    let after = h.core.fetches.lock().get(&root).unwrap().last_try;
    assert!(after > before, "fetched again");
}

#[tokio::test]
async fn the_background_fetch_goes_by_the_users_configuration_of_git() {
    let h = harness("fetch-user-config");
    let (p, r) = h.project(false).await;
    let (bare, other) = with_remote(&h.dir, &r);
    // An address only the user's configuration knows how to reach (`insteadOf`, as a proxy or
    // a mirror would be set): nothing of Escouade's stands in its way.
    git(&r, &["config", &format!("url.{bare}.insteadOf"), "team:"]);
    git(&r, &["remote", "set-url", "origin", "team:"]);
    push_theirs(&other, 2);
    h.core.fetch_all().await;
    let i = info(&h, &p).await;
    assert_eq!((i.fetch_failure, i.behind), (None, 1));
    assert!(
        !i.fetch_ssh_note,
        "no ssh of Windows for a remote that is no ssh"
    );
}

#[test]
fn only_the_stalling_git_for_windows_takes_windows_ssh_and_never_over_the_users() {
    use crate::git::{is_ssh_url, ssh_stalls, windows_ssh_command};
    assert!(ssh_stalls("git version 2.47.0.windows.1\n"));
    for v in [
        "git version 2.47.0.windows.2",
        "git version 2.47.1.windows.1",
        "git version 2.43.0",
    ] {
        assert!(!ssh_stalls(v), "{v}");
    }
    for u in [
        "git@github.com:me/x.git",
        "ssh://git@github.com/me/x.git",
        "git+ssh://host/x",
        "host:x",
    ] {
        assert!(is_ssh_url(u), "{u}");
    }
    for u in [
        "https://github.com/me/x.git",
        "C:/repos/x.git",
        r"C:\repos\x.git",
        "/srv/x.git",
        "../x.git",
        "file:///x",
        "x.git",
    ] {
        assert!(!is_ssh_url(u), "{u}");
    }
    let ssh = Path::new(r"C:\Windows\System32\OpenSSH\ssh.exe");
    let urls = vec!["git@github.com:me/x.git".to_string()];
    let broken = "git version 2.47.0.windows.1";
    assert_eq!(
        windows_ssh_command(broken, false, &urls, Some(ssh)).as_deref(),
        Some("\"C:/Windows/System32/OpenSSH/ssh.exe\" -o BatchMode=yes")
    );
    // The user's own ssh, a good git, no ssh remote, no ssh of Windows: nothing changes.
    assert_eq!(windows_ssh_command(broken, true, &urls, Some(ssh)), None);
    assert_eq!(
        windows_ssh_command("git version 2.51.0.windows.1", false, &urls, Some(ssh)),
        None
    );
    let https = vec!["https://github.com/me/x.git".to_string()];
    assert_eq!(windows_ssh_command(broken, false, &https, Some(ssh)), None);
    assert_eq!(windows_ssh_command(broken, false, &urls, None), None);
}

#[test]
fn a_failure_is_told_by_its_reason() {
    use crate::git::fetch_failure_kind as kind;
    use FetchFailureKind::*;
    for (said, k) in [
        ("git@github.com: Permission denied (publickey).", Credentials),
        (
            "fatal: Cannot prompt because user interactivity has been disabled.",
            Credentials,
        ),
        (
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
            Credentials,
        ),
        ("Host key verification failed.", Credentials),
        ("ssh: connect to host github.com port 22: Connection timed out", Network),
        (
            "fatal: unable to access 'https://github.com/a/b.git/': Failed to connect to github.com port 443 after 21101 ms: Could not connect to server",
            Network,
        ),
        ("ssh: Could not resolve hostname github.com: No such host is known.", Network),
        ("fetch-pack: unexpected disconnect while reading sideband packet", Network),
        (
            "fatal: '/x' does not appear to be a git repository",
            Other,
        ),
    ] {
        assert_eq!(kind(said, false), k, "{said}");
    }
    assert_eq!(kind("anything", true), Timeout);
}

/// The measure of ESC-561 on a real repository behind its remote (`ESC561_REPRO`, a clone of
/// this one eight days behind): how long the background fetch takes, and through which ssh.
/// Run by hand: `ESC561_REPRO=<dir> cargo nextest run --run-ignored only measure_`.
#[tokio::test]
#[ignore]
async fn measure_the_background_fetch_of_a_repository_behind() {
    let Ok(repo) = std::env::var("ESC561_REPRO") else {
        return;
    };
    let start = Instant::now();
    let run = crate::git::fetch_background(&repo, Duration::from_secs(300), || true).await;
    println!(
        "ESC-561 measure: {:?} in {:.1} s, windows ssh: {}, timed out: {}",
        run.result.as_ref().map_err(|e| format!("{e:#}")),
        start.elapsed().as_secs_f64(),
        run.windows_ssh,
        run.timed_out
    );
    assert!(run.result.is_ok());
}

#[tokio::test]
async fn a_pass_got_ready_before_the_setting_was_turned_off_runs_nothing() {
    let h = harness("fetch-race");
    let (p, r) = h.project(false).await;
    let (_, other) = with_remote(&h.dir, &r);
    let root = root(&h, &p).await;
    // Its roots read (the look of `fetch_all`), then the setting turned off before its git starts.
    h.core.fetch_plans().await;
    push_theirs(&other, 2);
    h.set_settings(|s| s.git_fetch.auto = false);
    h.core.fetch_root(&root, 300).await;
    let i = info(&h, &p).await;
    assert_eq!((i.behind, i.last_fetch), (0, None), "nothing fetched");
    // The project's « Toujours » still wants it.
    set_project(
        &h,
        &p,
        ProjectFetch {
            mode: FetchMode::Always,
            ..Default::default()
        },
    );
    h.core.fetch_root(&root, 300).await;
    assert_eq!(info(&h, &p).await.behind, 1);
}
