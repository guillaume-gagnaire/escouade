//! A fake HTTP server for the tests of the plugins' clients, in their crates: canned answers by method and path, and every
//! request kept to be looked at. The part of `src-tauri/src/integrations/fake.rs` a client's own tests need.
#![allow(dead_code)]

use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    /// The path and its query, as sent (`/rest/api/3/myself`, `/1/cards/c1?idList=l2`).
    pub target: String,
    /// Header names in lower case.
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Request {
    pub fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or("")
    }

    /// A query parameter, decoded (its name too: `assignees[]` is sent `assignees%5B%5D`).
    pub fn query(&self, name: &str) -> Option<String> {
        let q = self.target.split_once('?')?.1;
        q.split('&').find_map(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            (decode(k) == name).then(|| decode(v))
        })
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

struct Route {
    method: String,
    /// A path (any query), or a path and its query.
    target: String,
    status: u16,
    body: String,
    /// Headers of the answer besides its type and length (GitHub's `Link`).
    headers: Vec<(String, String)>,
    /// Answers the next request only (`on_once`): the route before it answers the ones after.
    once: bool,
}

impl Route {
    fn matches(&self, req: &Request) -> bool {
        // `/graphql#<operation>`: a GraphQL operation, by the `operationName` of its body;
        // `/graphql#<operation>@<owner>`: of that owner's repository only (`variables.owner`).
        if let Some((path, op)) = self.target.split_once('#') {
            let (op, owner) = match op.split_once('@') {
                Some((op, owner)) => (op, Some(owner)),
                None => (op, None),
            };
            let body = req.json();
            return self.method == req.method
                && path == req.path()
                && body["operationName"].as_str() == Some(op)
                && owner.is_none_or(|o| body["variables"]["owner"].as_str() == Some(o));
        }
        self.method == req.method
            && if self.target.contains('?') {
                self.target == req.target
            } else {
                self.target == req.path()
            }
    }
}

#[derive(Clone)]
pub struct FakeServer {
    pub url: String,
    routes: Arc<Mutex<Vec<Route>>>,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl FakeServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = Self::at(format!("http://{}", listener.local_addr().unwrap()));
        let s = server.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let s = s.clone();
                tokio::spawn(async move { s.serve(stream).await });
            }
        });
        server
    }

    fn at(url: String) -> Self {
        Self {
            url,
            routes: Arc::default(),
            requests: Arc::default(),
        }
    }

    /// One request read from `stream`, answered as its route says; then the connection closes.
    async fn serve<S: AsyncRead + AsyncWrite + Unpin>(&self, mut stream: S) {
        let Some(req) = read_request(&mut stream).await else {
            return;
        };
        let (status, body, headers) = self.answer(&req);
        self.requests.lock().push(req);
        let extra: String = headers
            .iter()
            .map(|(k, v)| format!("{k}: {v}\r\n"))
            .collect();
        let head = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes()).await;
        let _ = stream.write_all(body.as_bytes()).await;
        let _ = stream.shutdown().await;
    }

    /// Answers `method target` (the last route set for it wins) with `status` and `body`.
    pub fn on(&self, method: &str, target: &str, status: u16, body: Value) {
        self.on_with(method, target, status, body, &[]);
    }

    /// `on`, the answer carrying `headers` too.
    pub fn on_with(
        &self,
        method: &str,
        target: &str,
        status: u16,
        body: Value,
        headers: &[(&str, &str)],
    ) {
        self.routes.lock().push(Route {
            method: method.into(),
            target: target.into(),
            status,
            body: if body.is_null() {
                String::new()
            } else {
                body.to_string()
            },
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            once: false,
        });
    }

    /// `on_with`, for the next request only: the route set before it answers the ones after.
    pub fn on_once(
        &self,
        method: &str,
        target: &str,
        status: u16,
        body: Value,
        headers: &[(&str, &str)],
    ) {
        self.on_with(method, target, status, body, headers);
        if let Some(route) = self.routes.lock().last_mut() {
            route.once = true;
        }
    }

    /// Answers `method target` with `text` as it is (a log, not JSON), carrying `headers`.
    pub fn on_text(
        &self,
        method: &str,
        target: &str,
        status: u16,
        text: &str,
        headers: &[(&str, &str)],
    ) {
        self.routes.lock().push(Route {
            method: method.into(),
            target: target.into(),
            status,
            body: text.to_string(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            once: false,
        });
    }

    fn answer(&self, req: &Request) -> (u16, String, Vec<(String, String)>) {
        let mut routes = self.routes.lock();
        // The last route set for the request wins; one set `on_once` is used up by it.
        let Some(i) = routes.iter().rposition(|r| r.matches(req)) else {
            return (404, r#"{"message":"Not Found"}"#.into(), Vec::new());
        };
        let route = &routes[i];
        let (once, answer) = (
            route.once,
            (route.status, route.body.clone(), route.headers.clone()),
        );
        if once {
            routes.remove(i);
        }
        answer
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().clone()
    }

    /// The requests other than reads (GET).
    pub fn writes(&self) -> Vec<Request> {
        self.requests()
            .into_iter()
            .filter(|r| r.method != "GET")
            .collect()
    }
}

async fn read_request<S: AsyncRead + Unpin>(stream: &mut S) -> Option<Request> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let target = first.next()?.to_string();
    let headers: HashMap<String, String> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))
        .collect();
    let len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    while buf.len() < head_end + len {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = String::from_utf8_lossy(&buf[head_end..(head_end + len).min(buf.len())]).to_string();
    Some(Request {
        method,
        target,
        headers,
        body,
    })
}
