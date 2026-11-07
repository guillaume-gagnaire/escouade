//! `escouade-hook`: the hook of a harness that runs its hooks as commands (Codex, Copilot CLI),
//! for its interface in a terminal (« Vue <harness> », `engine::terminal`). It reads the event on
//! its input and posts it to Escouade's hooks' server, as Claude Code's HTTP hooks do: to the
//! address of `ESCOUADE_HOOK_URL`, with the token of `ESCOUADE_HOOK_TOKEN`, both given to the
//! interface Escouade starts.
//!
//! Its argument, when it has one, is the name of the event (Copilot's payloads do not say it):
//! posted as `?event=<name>`, a name of letters and digits only.
//!
//! Without the token it exits at once and does nothing: a hook declared where the user's own
//! sessions read it too leaves them alone. It always exits with 0 and writes nothing but the line of
//! context Escouade may answer (none since ESC-310), whatever
//! happens: the harness goes on as without it, and decides nothing on its word.
//!
//! Only the standard library, so that it starts fast and weighs little.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// The variables it reads.
const TOKEN_VAR: &str = "ESCOUADE_HOOK_TOKEN";
const URL_VAR: &str = "ESCOUADE_HOOK_URL";
/// The most of an event it sends (the server takes no more): a tool's input, a long prompt.
const MAX_BODY: u64 = 4 << 20;
/// How long it may take to reach the server, and to send and hear back.
const TIMEOUT: Duration = Duration::from_secs(5);

fn main() {
    let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
    if let Some(token) = var(TOKEN_VAR) {
        if let Some(url) = var(URL_VAR) {
            let url = with_event(&url, std::env::args().nth(1).as_deref());
            if let Some((200, body)) = post(&url, &token, std::io::stdin().lock()) {
                // Only what Escouade adds to the context: its plain `{}` says nothing.
                if body.starts_with('{') && body != "{}" {
                    let _ = writeln!(std::io::stdout(), "{body}");
                }
            }
        }
    }
    std::process::exit(0);
}

/// The address `url` with the event `event` in its query; as it is with none, or with a name that
/// is not letters and digits only.
fn with_event(url: &str, event: Option<&str>) -> String {
    match event.filter(|e| !e.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric())) {
        Some(e) => format!("{url}?event={e}"),
        None => url.to_string(),
    }
}

/// Where an address of the hooks' server points: `http://127.0.0.1:<port><path>` (the loopback
/// only, its port, a path or `/`).
#[derive(Debug, PartialEq, Eq)]
struct Target {
    addr: SocketAddr,
    host: String,
    path: String,
}

fn target(url: &str) -> Option<Target> {
    let rest = url.trim().strip_prefix("http://")?;
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let addr: SocketAddr = host.parse().ok()?;
    addr.ip().is_loopback().then(|| Target {
        addr,
        host: host.to_string(),
        path: path.to_string(),
    })
}

/// The request that posts `body` to `t` with `token`.
fn request(t: &Target, token: &str, body: &[u8]) -> Vec<u8> {
    let mut r = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        t.path,
        t.host,
        token.trim(),
        body.len()
    )
    .into_bytes();
    r.extend_from_slice(body);
    r
}

/// Posts what `input` holds (`MAX_BODY` at most) to `url` with `token`: the status the server
/// answered and its body, None when it could not be reached or the address is not the loopback's.
fn post(url: &str, token: &str, input: impl Read) -> Option<(u16, String)> {
    let t = target(url)?;
    let mut body = Vec::new();
    input.take(MAX_BODY).read_to_end(&mut body).ok()?;
    let mut stream = TcpStream::connect_timeout(&t.addr, TIMEOUT).ok()?;
    stream.set_read_timeout(Some(TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(TIMEOUT)).ok()?;
    stream.write_all(&request(&t, token, &body)).ok()?;
    let mut answer = Vec::new();
    let _ = stream.take(16 << 10).read_to_end(&mut answer);
    let text = String::from_utf8_lossy(&answer);
    let status = text.split_whitespace().nth(1)?.parse().ok()?;
    let body = text.split_once("\r\n\r\n").map_or("", |(_, b)| b.trim());
    Some((status, body.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn only_the_loopback_is_a_target() {
        assert_eq!(
            target("http://127.0.0.1:4120/hook"),
            Some(Target {
                addr: "127.0.0.1:4120".parse().unwrap(),
                host: "127.0.0.1:4120".into(),
                path: "/hook".into(),
            })
        );
        assert_eq!(target("http://127.0.0.1:80").unwrap().path, "/");
        assert!(target("http://[::1]:4120/hook").is_some());
        for refused in [
            "https://127.0.0.1:4120/hook",
            "http://10.0.0.2:4120/hook",
            "http://example.com/hook",
            "http://localhost:4120/hook",
            "",
        ] {
            assert_eq!(target(refused), None, "{refused}");
        }
    }

    #[test]
    fn the_request_carries_the_token_and_the_whole_event() {
        let t = target("http://127.0.0.1:4120/hook").unwrap();
        let r = String::from_utf8(request(&t, " tok ", b"{\"a\":1}")).unwrap();
        assert!(r.starts_with("POST /hook HTTP/1.1\r\n"));
        assert!(r.contains("\r\nHost: 127.0.0.1:4120\r\n"));
        assert!(r.contains("\r\nAuthorization: Bearer tok\r\n"));
        assert!(r.contains("\r\nContent-Length: 7\r\n"));
        assert!(r.ends_with("\r\n\r\n{\"a\":1}"));
    }

    /// A server that answers one request with `status`, and gives what it received.
    fn one_request(status: u16) -> (String, std::thread::JoinHandle<String>) {
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
    fn the_event_named_by_its_argument_goes_in_the_query() {
        let url = "http://127.0.0.1:4120/hook";
        assert_eq!(
            with_event(url, Some("agentStop")),
            "http://127.0.0.1:4120/hook?event=agentStop"
        );
        assert_eq!(with_event(url, None), url);
        for refused in ["", "a&b=c", "stop now", "é"] {
            assert_eq!(with_event(url, Some(refused)), url, "{refused}");
        }
        let t = target(&with_event(url, Some("agentStop"))).unwrap();
        assert_eq!(t.path, "/hook?event=agentStop");
    }

    #[test]
    fn it_posts_what_it_reads_and_tells_the_answer() {
        let (url, served) = one_request(200);
        let event = br#"{"hook_event_name":"Stop","session_id":"s1"}"#;
        assert_eq!(post(&url, "tok", &event[..]), Some((200, "{}".to_string())));
        let got = served.join().unwrap();
        assert!(got.contains("Authorization: Bearer tok"));
        assert!(got.ends_with(std::str::from_utf8(event).unwrap()));
    }

    #[test]
    fn an_unreachable_server_is_no_failure() {
        // A port nobody listens on any more.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        assert_eq!(
            post(&format!("http://127.0.0.1:{port}/hook"), "tok", &b"{}"[..]),
            None
        );
    }
}
