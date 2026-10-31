//! `escouade-hook` (`src/bin/escouade-hook.rs`) as a harness runs it: the event on its input,
//! the address and the token in its environment.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::Duration;

const TOKEN_VAR: &str = "ESCOUADE_HOOK_TOKEN";
const URL_VAR: &str = "ESCOUADE_HOOK_URL";

/// The program, with none of the variables it reads.
fn program() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_escouade-hook"));
    c.env_remove(TOKEN_VAR)
        .env_remove(URL_VAR)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    c
}

/// Runs it with `input` on its input.
fn run(mut c: Command, input: &[u8]) -> Output {
    let mut child = c.spawn().unwrap();
    // It may exit before it reads (no token): a closed pipe is no failure here.
    let _ = child.stdin.take().unwrap().write_all(input);
    child.wait_with_output().unwrap()
}

/// A server that answers one request with `status`: its address, and what it received.
fn one_request(status: u16) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/hook", listener.local_addr().unwrap());
    let served = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut got = Vec::new();
        let mut buf = [0u8; 4096];
        // Up to the end of its body, by its length.
        loop {
            let n = s.read(&mut buf).unwrap_or(0);
            got.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&got).to_string();
            if let Some(end) = text.find("\r\n\r\n") {
                let len: usize = text
                    .lines()
                    .find_map(|l| l.strip_prefix("Content-Length: "))
                    .and_then(|l| l.trim().parse().ok())
                    .unwrap_or(0);
                if got.len() >= end + 4 + len {
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        let _ = write!(s, "HTTP/1.1 {status} OK\r\nContent-Length: 2\r\n\r\n{{}}");
        String::from_utf8_lossy(&got).into_owned()
    });
    (url, served)
}

#[test]
fn it_posts_the_event_with_the_token() {
    let (url, served) = one_request(200);
    let mut c = program();
    c.env(TOKEN_VAR, "tok-1").env(URL_VAR, &url);
    let event = r#"{"hook_event_name":"UserPromptSubmit","session_id":"s1"}"#;
    let out = run(c, event.as_bytes());
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    let got = served.join().unwrap();
    assert!(got.starts_with("POST /hook HTTP/1.1"), "{got}");
    assert!(got.contains("Authorization: Bearer tok-1"), "{got}");
    assert!(got.ends_with(event), "{got}");
}

#[test]
fn without_the_token_it_does_nothing() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/hook", listener.local_addr().unwrap());
    for token in [None, Some(""), Some("  ")] {
        let mut c = program();
        c.env(URL_VAR, &url);
        if let Some(t) = token {
            c.env(TOKEN_VAR, t);
        }
        let out = run(c, b"{}");
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
    // Nobody called.
    assert!(listener.accept().is_err());
}

#[test]
fn a_refusal_or_no_server_still_ends_quietly() {
    let (url, served) = one_request(401);
    let mut c = program();
    c.env(TOKEN_VAR, "wrong").env(URL_VAR, &url);
    let out = run(c, b"{}");
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    served.join().unwrap();
    // Not the loopback, or nothing there.
    let gone = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    for url in [
        "http://example.com/hook".to_string(),
        format!("http://127.0.0.1:{gone}/hook"),
    ] {
        let mut c = program();
        c.env(TOKEN_VAR, "tok").env(URL_VAR, &url);
        let out = run(c, b"{}");
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
}

#[test]
fn the_event_its_argument_names_goes_in_the_query() {
    let (url, served) = one_request(200);
    let mut c = program();
    c.arg("agentStop")
        .env(TOKEN_VAR, "tok-2")
        .env(URL_VAR, &url)
        // What Copilot's hooks get from its process: never a reason to post anything else.
        .env("GH_TOKEN", "gho_not_a_token");
    let event = r#"{"sessionId":"s1","stopReason":"end_turn"}"#;
    let out = run(c, event.as_bytes());
    assert_eq!(out.status.code(), Some(0));
    let got = served.join().unwrap();
    assert!(
        got.starts_with("POST /hook?event=agentStop HTTP/1.1"),
        "{got}"
    );
    assert!(got.ends_with(event), "{got}");
    assert!(!got.contains("gho_not_a_token"), "{got}");
}

#[test]
fn the_context_escouade_answers_is_written_for_the_harness() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/hook", listener.local_addr().unwrap());
    let advice = r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"a line of context"}}"#;
    let served = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut buf = [0u8; 4096];
        let _ = s.read(&mut buf);
        let _ = write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{advice}",
            advice.len()
        );
    });
    let mut c = program();
    c.env(TOKEN_VAR, "tok").env(URL_VAR, &url);
    let out = run(c, br#"{"hook_event_name":"PreToolUse","tool_name":"Grep"}"#);
    served.join().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), advice);
    assert!(out.stderr.is_empty());
}
