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
fn help_wins_over_the_other_options() {
    // Whatever is around it, and wherever it is: help prints and nothing runs.
    help_of("init", "--help", &["--data", "somewhere"]);
    help_of("recovery-token", "-h", &["--team", "nothex"]);
}

#[test]
fn help_command_prints_the_help_of_a_subcommand() {
    for command in ["init", "serve", "recovery-token"] {
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
        assert!(stdout.contains("recovery-token") && stdout.contains("USAGE"));
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
