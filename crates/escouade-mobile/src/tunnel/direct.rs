//! A space reached without the tunnel (ESC-570): the code typed with an address, when the phone reaches the core's API
//! itself (its address on Tailscale, the e2e's local server). The same local proxy as the tunnel's, on `127.0.0.1`: it
//! forwards a connection only if its first line names the session's secret ([`super::gate`]), writes the device's
//! token in the request, and carries the bytes to the address, a thread per connection (a handful at most: the app's
//! WebSocket). The WebView never holds the token.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use zeroize::Zeroizing;

use super::gate::{self, Gate};
use super::{TunnelError, TunnelState, TunnelStatus, API_PATH};
use crate::keys::{encode_key, random_key};

/// How long a connection has to name the secret, and to reach the address.
const GATE_WAIT: Duration = Duration::from_secs(10);
/// The connections at once.
const MAX_CONNECTIONS: u32 = 16;

struct Inner {
    target: Vec<SocketAddr>,
    secret: String,
    token: Option<Zeroizing<String>>,
    stop: AtomicBool,
    connections: AtomicU32,
}

/// The proxy to an address.
pub struct Direct {
    inner: Arc<Inner>,
    proxy: SocketAddr,
}

impl Direct {
    /// The proxy to `api` (`host:port`, resolved now), the device's token written in each request.
    pub fn start(api: &str, device_token: Option<String>) -> Result<Direct, TunnelError> {
        let target: Vec<SocketAddr> = api
            .trim()
            .to_socket_addrs()
            .map_err(|e| TunnelError::BadApi(format!("{api}: {e}")))?
            .collect();
        if target.is_empty() {
            return Err(TunnelError::BadApi(api.to_string()));
        }
        let listener =
            TcpListener::bind("127.0.0.1:0").map_err(|e| TunnelError::Io(e.to_string()))?;
        let proxy = listener
            .local_addr()
            .map_err(|e| TunnelError::Io(e.to_string()))?;
        let inner = Arc::new(Inner {
            target,
            secret: encode_key(&random_key()),
            token: device_token
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .map(Zeroizing::new),
            stop: AtomicBool::new(false),
            connections: AtomicU32::new(0),
        });
        let accepting = inner.clone();
        std::thread::Builder::new()
            .name("escouade-direct".into())
            .spawn(move || accept(listener, accepting))
            .map_err(|e| TunnelError::Io(e.to_string()))?;
        Ok(Direct { inner, proxy })
    }

    pub fn status(&self) -> TunnelStatus {
        let stopped = self.inner.stop.load(Ordering::SeqCst);
        TunnelStatus {
            state: if stopped {
                TunnelState::Stopped
            } else {
                TunnelState::Up
            },
            proxy_url: format!("http://{}/{}", self.proxy, self.inner.secret),
            websocket_url: format!("ws://{}/{}{API_PATH}", self.proxy, self.inner.secret),
            last_handshake_secs: None,
            tx_bytes: 0,
            rx_bytes: 0,
            connections: self.inner.connections.load(Ordering::SeqCst),
            error: None,
        }
    }

    /// No new connection; the accepting thread ends. Idempotent.
    pub fn stop(&self) {
        if !self.inner.stop.swap(true, Ordering::SeqCst) {
            // Wakes the blocked accept.
            let _ = TcpStream::connect_timeout(&self.proxy, Duration::from_millis(200));
        }
    }
}

impl Drop for Direct {
    fn drop(&mut self) {
        self.stop();
    }
}

fn accept(listener: TcpListener, inner: Arc<Inner>) {
    for stream in listener.incoming() {
        if inner.stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else { continue };
        if inner.connections.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
            continue;
        }
        inner.connections.fetch_add(1, Ordering::SeqCst);
        let conn = inner.clone();
        let spawned = std::thread::Builder::new()
            .name("escouade-direct-conn".into())
            .spawn(move || {
                serve(stream, &conn);
                conn.connections.fetch_sub(1, Ordering::SeqCst);
            });
        if spawned.is_err() {
            inner.connections.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

fn serve(mut local: TcpStream, inner: &Inner) {
    let _ = local.set_read_timeout(Some(GATE_WAIT));
    let mut seen = Vec::new();
    let mut buf = [0u8; 4096];
    let first = loop {
        let n = match local.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        seen.extend_from_slice(&buf[..n]);
        match gate::check(
            &seen,
            &inner.secret,
            inner.token.as_deref().map(String::as_str),
        ) {
            Gate::NeedMore => continue,
            Gate::Refuse => {
                let _ = local.write_all(gate::REFUSAL);
                return;
            }
            Gate::Pass(bytes) => break bytes,
        }
    };
    let Some(mut remote) = inner
        .target
        .iter()
        .find_map(|a| TcpStream::connect_timeout(a, GATE_WAIT).ok())
    else {
        let _ = local.write_all(
            b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        return;
    };
    let _ = local.set_read_timeout(None);
    let _ = remote.set_nodelay(true);
    let _ = local.set_nodelay(true);
    if remote.write_all(&first).is_err() {
        return;
    }
    let (Ok(mut back_from), Ok(mut back_to)) = (remote.try_clone(), local.try_clone()) else {
        return;
    };
    let back = std::thread::spawn(move || {
        let _ = std::io::copy(&mut back_from, &mut back_to);
        let _ = back_to.shutdown(Shutdown::Write);
    });
    let _ = std::io::copy(&mut local, &mut remote);
    let _ = remote.shutdown(Shutdown::Write);
    let _ = back.join();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;

    /// A server of test: the first request it reads, then `answer`.
    fn server(answer: &'static [u8]) -> (SocketAddr, std::sync::mpsc::Receiver<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for s in l.incoming() {
                let mut s = s.unwrap();
                let mut reader = std::io::BufReader::new(s.try_clone().unwrap());
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    head.push_str(&line);
                }
                s.write_all(answer).unwrap();
                let _ = tx.send(head);
            }
        });
        (addr, rx)
    }

    fn get(proxy: &str, path: &str) -> String {
        let addr = proxy
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap();
        let mut s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    }

    #[test]
    fn forwards_with_the_secret_writes_the_token_and_refuses_without_it() {
        let (addr, heads) =
            server(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        let direct = Direct::start(&addr.to_string(), Some("tok-1".into())).unwrap();
        let status = direct.status();
        assert_eq!(status.state, TunnelState::Up);
        let secret = status.proxy_url.rsplit('/').next().unwrap().to_string();
        assert!(get(&status.proxy_url, &format!("/{secret}/remote")).ends_with("ok"));
        let head = heads.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(head.starts_with("GET /remote HTTP/1.1\r\n"), "{head}");
        assert!(head.contains("X-Escouade-Device: tok-1\r\n"), "{head}");
        assert!(!head.contains(&secret), "{head}");
        // Without the secret: refused, nothing reaches the server.
        assert!(get(&status.proxy_url, "/remote").starts_with("HTTP/1.1 403"));
        assert!(heads.recv_timeout(Duration::from_millis(300)).is_err());
        direct.stop();
        assert_eq!(direct.status().state, TunnelState::Stopped);
    }

    #[test]
    fn an_address_that_is_none_is_refused() {
        assert!(matches!(
            Direct::start("nowhere", None),
            Err(TunnelError::BadApi(_))
        ));
    }
}
