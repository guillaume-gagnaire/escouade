//! The command line: `--help` after a subcommand prints that subcommand's help and does nothing else;
//! an unknown or misplaced option is a usage error (exit 2), never ignored.

use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Runs the binary in `dir` (its working folder, so the default `./data` would land there). A run that
/// does not end within 20 s is killed and fails the test: `serve` would never end.
fn run(dir: &Path, args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_escouade-team-server"))
        .args(args)
        .current_dir(dir)
        .env_remove("ESCOUADE_TEAM_DATA")
        .env_remove("ESCOUADE_TEAM_LISTEN")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    let start = Instant::now();
    loop {
        if child.try_wait().expect("wait").is_some() {
            return child.wait_with_output().expect("output");
        }
        if start.elapsed() > Duration::from_secs(20) {
            let _ = child.kill();
            panic!("{args:?} did not end: it ran the command");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn assert_nothing_created(dir: &tempfile::TempDir) {
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(left.is_empty(), "files were created: {left:?}");
}

fn help_of(command: &str, flag: &str, extra: &[&str]) {
    let dir = tempfile::tempdir().unwrap();
    let port = free_port();
    let listen = format!("127.0.0.1:{port}");
    let mut args = vec![command, flag];
    args.extend_from_slice(extra);
    if command == "serve" {
        args.extend(["--listen", &listen]);
    }
    let out = run(dir.path(), &args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{args:?}: {}",
        text(&out.stderr)
    );
    let stdout = text(&out.stdout);
    assert!(
        stdout.contains(&format!("escouade-team-server {command}:")),
        "{args:?} prints the help of {command}: {stdout}"
    );
    assert!(
        stdout.contains("ESCOUADE_TEAM_DATA"),
        "the variables are named"
    );
    assert!(stdout.contains("--help"), "the help names --help");
    assert!(
        out.stderr.is_empty(),
        "nothing on stderr: {}",
        text(&out.stderr)
    );
    assert_nothing_created(&dir);
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "a port was opened"
    );
}

#[test]
fn init_help_creates_nothing() {
    help_of("init", "--help", &[]);
    help_of("init", "-h", &[]);
}

#[test]
fn serve_help_listens_nowhere_and_creates_nothing() {
    help_of("serve", "--help", &[]);
    help_of("serve", "-h", &[]);
}

#[test]
fn recovery_token_help_creates_nothing() {
    help_of("recovery-token", "--help", &[]);
    help_of("recovery-token", "-h", &[]);
}

#[test]
fn drop_entry_help_creates_nothing() {
    help_of("drop-entry", "--help", &[]);
    help_of("drop-entry", "-h", &["--team", "nothex", "--yes"]);
}

#[test]
fn help_wins_over_the_other_options() {
    // Whatever is around it, and wherever it is: help prints and nothing runs.
    help_of("init", "--help", &["--data", "somewhere"]);
    help_of("recovery-token", "-h", &["--team", "nothex"]);
}

#[test]
fn help_command_prints_the_help_of_a_subcommand() {
    for command in ["init", "serve", "recovery-token", "drop-entry"] {
        let dir = tempfile::tempdir().unwrap();
        let out = run(dir.path(), &["help", command]);
        assert_eq!(out.status.code(), Some(0));
        assert!(text(&out.stdout).contains(&format!("escouade-team-server {command}:")));
        assert_nothing_created(&dir);
    }
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), &["help", "nope"]);
    assert_eq!(out.status.code(), Some(2));
    assert_nothing_created(&dir);
}

#[test]
fn global_help_and_no_argument_list_the_commands() {
    for args in [&["--help"][..], &["-h"], &["help"], &[]] {
        let dir = tempfile::tempdir().unwrap();
        let out = run(dir.path(), args);
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        let stdout = text(&out.stdout);
        assert!(
            stdout.contains("recovery-token")
                && stdout.contains("USAGE")
                && stdout.contains("drop-entry")
        );
        assert_nothing_created(&dir);
    }
}

#[test]
fn version_everywhere_runs_nothing() {
    for args in [
        &["--version"][..],
        &["-V"],
        &["init", "--version"],
        &["serve", "-V"],
        &["recovery-token", "--version"],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let out = run(dir.path(), args);
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        assert_eq!(
            text(&out.stdout),
            format!("escouade-team-server {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert_nothing_created(&dir);
    }
}

#[test]
fn an_unknown_or_misplaced_option_is_a_usage_error() {
    let cases: &[&[&str]] = &[
        &["init", "--bogus"],
        &["init", "--bogus=1"],
        &["init", "-x"],
        &["init", "stray"],
        &["init", "--listen", "127.0.0.1:1"], // a serve option, misplaced
        &["init", "--trust-proxy"],
        &["init", "--team", "00"],
        &["serve", "--bogus"],
        &["serve", "--team", "00"], // a recovery-token option, misplaced
        &["serve", "--trust-proxy=1"],
        &["serve", "stray"],
        &["recovery-token", "--bogus", "--team", "00"],
        &["recovery-token", "--listen", "127.0.0.1:1"],
        &["drop-entry", "--listen", "127.0.0.1:1"],
        &["drop-entry", "--yes=1"],
        &["drop-entry", "--team", "00", "stray"],
        &["init", "--yes"],
        &["--data", "x", "init"], // an option before the command
        &["--bogus"],
        &["bogus"],
        &["init", "--data"], // a value is missing
    ];
    for args in cases {
        let dir = tempfile::tempdir().unwrap();
        let out = run(dir.path(), args);
        assert_eq!(out.status.code(), Some(2), "{args:?} is a usage error");
        assert!(
            text(&out.stderr).contains("escouade-team-server:"),
            "{args:?} says why: {}",
            text(&out.stderr)
        );
        assert!(out.stdout.is_empty(), "{args:?}: no output on stdout");
        assert_nothing_created(&dir);
    }
}

#[test]
fn init_still_creates_the_base_and_prints_a_token() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), &["init", "--data", "base"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("The base is ready."));
    assert!(dir.path().join("base").join("team.db").exists());
    // `--opt=value` and the environment still work.
    let out = run(dir.path(), &["init", "--data=other"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(dir.path().join("other").join("team.db").exists());
}

// === drop-entry (SR2-02): an entry that blocks a team goes, after a confirmation, with its line of audit ===

mod common;

use common::{Backend, TestServer};

/// Runs the binary with `input` on its stdin (the answer to the confirmation).
fn run_with(dir: &Path, args: &[&str], input: &str) -> Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_escouade-team-server"))
        .args(args)
        .current_dir(dir)
        .env_remove("ESCOUADE_TEAM_DATA")
        .env_remove("ESCOUADE_TEAM_ID")
        .env_remove("ESCOUADE_TEAM_OPERATOR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().expect("wait").is_some() {
            return child.wait_with_output().expect("output");
        }
        if start.elapsed() > Duration::from_secs(20) {
            let _ = child.kill();
            panic!("{args:?} did not end");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A team on a SQLite base in its folder: the administrator's commit (1), a message (2), and a member's commit of
/// bytes no device reads that takes the next epoch (3).
async fn a_blocked_team() -> (TestServer, common::Device, String) {
    let s = TestServer::start(&Backend::Sqlite).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    admin.commit(0, b"admin: epoch 0 -> 1").await.ok();
    admin.message(b"a message").await;
    bob.commit_own(1, b"garbage").await.ok();
    let team = hex::encode(&admin.team);
    (s, admin, team)
}

fn data(s: &TestServer) -> String {
    s.dir.path().to_string_lossy().into_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drop_entry_refuses_without_the_team_the_stream_or_the_entry() {
    let (s, admin, team) = a_blocked_team().await;
    let d = data(&s);
    let cases: &[(&[&str], &str)] = &[
        (&["--stream", "team", "--seq", "3"], "--team"),
        (
            &["--team", "zz", "--stream", "team", "--seq", "3"],
            "--team",
        ),
        (&["--team", &team, "--seq", "3"], "needs the stream"),
        (
            &[
                "--team",
                &team,
                "--stream",
                "team",
                "--project",
                &team,
                "--seq",
                "3",
            ],
            "give one of them",
        ),
        (
            &["--team", &team, "--stream", "nope", "--seq", "3"],
            "escouade/project",
        ),
        (&["--team", &team, "--stream", "team"], "--seq"),
        (
            &["--team", &team, "--stream", "team", "--seq", "0"],
            "--seq",
        ),
        (
            &["--team", &team, "--stream", "team", "--seq", "9"],
            "no entry 9",
        ),
        (
            &["--team", &"0".repeat(32), "--stream", "team", "--seq", "3"],
            "no team",
        ),
        // A commit another follows: remove the later ones first.
        (
            &["--team", &team, "--stream", "team", "--seq", "1", "--yes"],
            "later commit",
        ),
    ];
    for (extra, why) in cases {
        let mut args = vec!["drop-entry", "--data", &d];
        args.extend_from_slice(extra);
        let out = run_with(s.dir.path(), &args, "");
        assert_eq!(
            out.status.code(),
            Some(1),
            "{args:?}: {}",
            text(&out.stderr)
        );
        assert!(
            text(&out.stderr).contains(why),
            "{args:?} says {why}: {}",
            text(&out.stderr)
        );
    }
    // Not confirmed (nothing typed, or something else than yes): nothing goes.
    for answer in ["", "no\n", "y\n"] {
        let out = run_with(
            s.dir.path(),
            &[
                "drop-entry",
                "--data",
                &d,
                "--team",
                &team,
                "--stream",
                "team",
                "--seq",
                "3",
            ],
            answer,
        );
        assert_eq!(out.status.code(), Some(1));
        assert!(
            text(&out.stdout).contains("a commit, for epoch 1"),
            "the entry is shown first"
        );
        assert!(text(&out.stderr).contains("not confirmed"));
    }
    assert_eq!(admin.read_team(0).await.entries.len(), 3, "nothing went");
    let team_id: [u8; 16] = admin.team.clone().try_into().unwrap();
    assert!(s
        .server
        .app()
        .store
        .dropped(team_id)
        .await
        .unwrap()
        .is_empty());
    s.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drop_entry_removes_the_blocking_commit_and_keeps_its_line_of_audit() {
    let (s, admin, team) = a_blocked_team().await;
    let d = data(&s);
    let served = admin.read_team(2).await.entries.remove(0);
    let digest = hex::encode(escouade_team_server::db::entry_digest(&served));
    let out = run_with(
        s.dir.path(),
        &[
            "drop-entry",
            "--data",
            &d,
            "--team",
            &team,
            "--stream",
            "team",
            "--seq",
            "3",
            "--by",
            "alice (ops)",
        ],
        "yes\n",
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains(&digest), "the digest is shown: {stdout}");
    assert!(stdout.contains("Removed"));
    let log = stdout
        .lines()
        .find(|l| l.contains("an entry removed by the operator"))
        .unwrap_or_default();
    assert!(
        !log.contains(&team) && !log.contains(&digest),
        "no id in the log: {log}"
    );
    assert!(
        !stdout.contains("garbage") && !text(&out.stderr).contains("garbage"),
        "never its bytes"
    );
    assert!(
        stdout.contains("an entry removed by the operator") && stdout.contains("\"drop-entry\""),
        "the audit's line in the log: {stdout}"
    );
    // The hole shows, and the epoch is free again for the administrator's commit.
    let seqs: Vec<u64> = admin
        .read_team(0)
        .await
        .entries
        .iter()
        .map(|e| e.seq)
        .collect();
    assert_eq!(seqs, [1, 2]);
    assert_eq!(admin.commit(1, b"admin: epoch 1 -> 2").await.ok().seq, 4);
    let team_id: [u8; 16] = admin.team.clone().try_into().unwrap();
    let audit = s.server.app().store.dropped(team_id).await.unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].operator, "alice (ops)");
    assert_eq!((audit[0].project, audit[0].seq), (None, 3));
    assert_eq!(hex::encode(audit[0].digest), digest);
    assert!(audit[0].at > 0);
    // `--yes` does without the question, a team's own stream is named, a project by its id.
    let catalog = "escouade/project";
    admin
        .append_project(&catalog.as_bytes().try_into().unwrap(), b"an entry")
        .await
        .ok();
    let out = run_with(
        s.dir.path(),
        &[
            "drop-entry",
            "--data",
            &d,
            "--team",
            &team,
            "--stream",
            catalog,
            "--seq",
            "1",
            "--yes",
        ],
        "",
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let project = [5u8; 16];
    admin.append_project(&project, b"an entry").await.ok();
    let out = run_with(
        s.dir.path(),
        &[
            "drop-entry",
            "--data",
            &d,
            "--team",
            &team,
            "--project",
            &hex::encode(project),
            "--seq",
            "1",
            "--yes",
        ],
        "",
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let audit = s.server.app().store.dropped(team_id).await.unwrap();
    assert_eq!(
        audit.iter().map(|a| a.project).collect::<Vec<_>>(),
        [None, Some(*b"escouade/project"), Some(project)]
    );
    assert!(
        audit.iter().all(|a| !a.operator.is_empty()),
        "the account's name by default"
    );
    // The operator reads the audit again; the log of the removal never held the team's id.
    let out = run_with(
        s.dir.path(),
        &["drop-entry", "--data", &d, "--team", &team, "--list"],
        "",
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let list = text(&out.stdout);
    assert_eq!(list.lines().count(), 3, "{list}");
    assert!(
        list.contains("by alice (ops): team stream, entry 3 (commit, epoch 1"),
        "{list}"
    );
    assert!(list.contains(&digest));
    assert!(
        list.contains("escouade/project stream, entry 1 (message"),
        "{list}"
    );
    s.finish().await;
}
