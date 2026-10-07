//! A fake HTTP server for the tests of the external ticket systems: canned answers by method and
//! path, and every request kept to be looked at; over https too, with a self-signed certificate.

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

    /// A query parameter, decoded.
    pub fn query(&self, name: &str) -> Option<String> {
        let q = self.target.split_once('?')?.1;
        q.split('&').find_map(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            (k == name).then(|| decode(v))
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

    /// The same over https, with a self-signed certificate for 127.0.0.1 (as a corporate proxy's
    /// own, which no system trusts).
    pub async fn start_tls() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = Self::at(format!("https://{}", listener.local_addr().unwrap()));
        let made = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(made.key_pair.serialize_der().into());
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![made.cert.der().clone()], key)
        .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let s = server.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let (s, acceptor) = (s.clone(), acceptor.clone());
                tokio::spawn(async move {
                    // A client that refuses the certificate ends the handshake: nothing to serve.
                    if let Ok(tls) = acceptor.accept(stream).await {
                        s.serve(tls).await;
                    }
                });
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
        let (status, body) = self.answer(&req);
        self.requests.lock().push(req);
        let head = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes()).await;
        let _ = stream.write_all(body.as_bytes()).await;
        let _ = stream.shutdown().await;
    }

    /// Answers `method target` (the last route set for it wins) with `status` and `body`.
    pub fn on(&self, method: &str, target: &str, status: u16, body: Value) {
        self.routes.lock().push(Route {
            method: method.into(),
            target: target.into(),
            status,
            body: if body.is_null() {
                String::new()
            } else {
                body.to_string()
            },
        });
    }

    fn answer(&self, req: &Request) -> (u16, String) {
        let routes = self.routes.lock();
        routes
            .iter()
            .rev()
            .find(|r| {
                r.method == req.method
                    && if r.target.contains('?') {
                        r.target == req.target
                    } else {
                        r.target == req.path()
                    }
            })
            .map(|r| (r.status, r.body.clone()))
            .unwrap_or((404, r#"{"message":"Not Found"}"#.into()))
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
