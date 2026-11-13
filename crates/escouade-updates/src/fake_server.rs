//! A fake HTTP server for the tests of the updates: the answers are scripted by path and come in
//! the order of the requests (the last one repeats), their bodies are bytes and their headers
//! whatever the test wants, and every request is kept to be looked at.
//!
//! It is also the fake proxy: a client that goes through a proxy sends the request in absolute
//! form (`GET http://host:port/path`), and the script goes by the path alone. A tunnel
//! (`CONNECT host:port HTTP/1.1`) is scripted by `host:port`, whose answer is only a status.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// One answer.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// The connection is closed without a word.
    hang_up: bool,
}

impl Reply {
    /// An answer with no body.
    pub fn status(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            hang_up: false,
        }
    }

    /// No answer: the server closes the connection as soon as it has read the request.
    pub fn hang_up() -> Self {
        Self {
            hang_up: true,
            ..Self::status(0)
        }
    }

    pub fn json(status: u16, body: &str) -> Self {
        Self::status(status)
            .with("Content-Type", "application/json")
            .body(body.as_bytes())
    }

    pub fn text(status: u16, body: &str) -> Self {
        Self::status(status)
            .with("Content-Type", "text/plain")
            .body(body.as_bytes())
    }

    pub fn html(status: u16, body: &str) -> Self {
        Self::status(status)
            .with("Content-Type", "text/html; charset=utf-8")
            .body(body.as_bytes())
    }

    pub fn bytes(status: u16, body: &[u8]) -> Self {
        Self::status(status)
            .with("Content-Type", "application/octet-stream")
            .body(body)
    }

    pub fn redirect(to: &str) -> Self {
        Self::status(302).with("Location", to)
    }

    fn body(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self
    }

    /// The header `name` is `value` (replaced when there is one already).
    pub fn with(mut self, name: &str, value: &str) -> Self {
        self.headers.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
        self.headers.push((name.into(), value.into()));
        self
    }
}

/// A request received.
#[derive(Debug, Clone)]
pub struct Seen {
    /// The path alone, whether the request came in absolute form or not.
    pub path: String,
    /// Header names in lower case.
    pub headers: HashMap<String, String>,
}

struct Script {
    replies: Vec<Reply>,
    next: usize,
}

#[derive(Clone)]
pub struct Server {
    pub port: u16,
    scripts: Arc<Mutex<HashMap<String, Script>>>,
    seen: Arc<Mutex<Vec<Seen>>>,
    /// What accepts the connections, until `stop`.
    listening: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Server {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = Self {
            port,
            scripts: Arc::default(),
            seen: Arc::default(),
            listening: Arc::default(),
        };
        server.accept(listener);
        // `localhost` is tried as `::1` before `127.0.0.1`, and a port nothing listens on at `::1`
        // is refused in a second or two on Windows: the loopback of IPv6 answers too, where
        // there is one.
        if let Ok(listener) = TcpListener::bind(("::1", port)).await {
            server.accept(listener);
        }
        server
    }

    fn accept(&self, listener: TcpListener) {
        let s = self.clone();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let s = s.clone();
                tokio::spawn(async move { s.serve(stream).await });
            }
        });
        self.listening.lock().push(task);
    }

    /// The server goes away: its port refuses connections from now on (a proxy that is shut down).
    pub async fn stop(&self) {
        let tasks: Vec<_> = self.listening.lock().drain(..).collect();
        for task in tasks {
            task.abort();
            // Awaited, so that the listener is closed when this returns.
            let _ = task.await;
        }
    }

    /// Where `path` is on this server.
    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    /// The same, as the proxy's address.
    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// The answers to `path`, one per request in this order, the last one for the rest.
    pub fn script(&self, path: &str, replies: Vec<Reply>) {
        assert!(!replies.is_empty(), "a script answers something");
        self.scripts
            .lock()
            .insert(path.into(), Script { replies, next: 0 });
    }

    /// Every request received, in order.
    pub fn seen(&self) -> Vec<Seen> {
        self.seen.lock().clone()
    }

    /// The requests received for `path`.
    pub fn hits(&self, path: &str) -> Vec<Seen> {
        self.seen().into_iter().filter(|s| s.path == path).collect()
    }

    fn answer(&self, path: &str) -> Reply {
        let mut scripts = self.scripts.lock();
        let Some(script) = scripts.get_mut(path) else {
            return Reply::status(404).with("Content-Type", "text/plain");
        };
        let reply = script.replies[script.next.min(script.replies.len() - 1)].clone();
        script.next += 1;
        reply
    }

    /// One request read from `stream`, answered as the script says; then the connection closes.
    async fn serve(&self, mut stream: tokio::net::TcpStream) {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let head_end = loop {
            let Ok(n) = stream.read(&mut chunk).await else {
                return;
            };
            if n == 0 {
                return;
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break i;
            }
        };
        let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
        let mut lines = head.lines();
        let target = lines
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_string();
        let headers = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))
            .collect();
        let path = path_of(&target);
        let reply = self.answer(&path);
        self.seen.lock().push(Seen { path, headers });
        if reply.hang_up {
            return;
        }
        let mut out = format!(
            "HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n",
            reply.status,
            reply.body.len()
        );
        for (name, value) in &reply.headers {
            out.push_str(&format!("{name}: {value}\r\n"));
        }
        out.push_str("\r\n");
        let _ = stream.write_all(out.as_bytes()).await;
        let _ = stream.write_all(&reply.body).await;
        let _ = stream.shutdown().await;
    }
}

/// The path of a request target, absolute form or not, its query left out.
fn path_of(target: &str) -> String {
    let rest = target
        .strip_prefix("http://")
        .map_or(target, |r| r.find('/').map_or("/", |i| &r[i..]));
    rest.split('?').next().unwrap_or("/").to_string()
}

/// A port something listens on, and hangs up on whoever comes: a server that is there and
/// answers nothing (a proxy that closes a tunnel without a word). Unlike a closed port, Windows
/// does not make the client wait for it.
pub async fn hangs_up() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { while listener.accept().await.is_ok() {} });
    port
}

/// A port nothing listens on: the connection is refused (Windows takes a second or two to say so,
/// retrying the SYN: only the tests of an unreachable proxy use it).
pub fn closed_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
