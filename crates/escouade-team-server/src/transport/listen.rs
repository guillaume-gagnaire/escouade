//! Listening: HTTP/1.1 (WebSocket upgrades included), in clear behind a reverse proxy, or in TLS by rustls with a
//! given certificate. Each request carries the peer's address (`ConnectInfo<SocketAddr>`).
//!
//! **Bounded** (SR2-07): so many connections open at once in all, and so many from one address (an IPv6 address by its
//! /64); past either, a new one is closed as soon as it is accepted. A connection where nothing passes, either way,
//! for `idle` is closed; the head of a request is read within 15 s, its body within its own time (`body.rs`). A
//! WebSocket counts in the connections until it closes.

use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use axum::extract::ConnectInfo;
use axum::Router;
use hyper::body::Incoming;
use hyper_util::rt::{TokioIo, TokioTimer};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tower::ServiceExt as _;

/// What `serve` bounds (SR2-07).
#[derive(Clone)]
pub struct Bounds {
    /// A connection where nothing is read nor written for so long is closed.
    pub idle: Duration,
    /// Connections open at once, in all.
    pub max_connections: usize,
    /// Connections open at once from one address; None behind a reverse proxy, where they all come from its address.
    pub per_address: Option<usize>,
    /// Called for each connection closed at once because a bound was reached (a metric).
    pub on_refused: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            idle: Duration::from_secs(60),
            max_connections: 8192,
            per_address: Some(512),
            on_refused: None,
        }
    }
}

#[derive(Default)]
struct Counts {
    total: usize,
    by_address: HashMap<IpAddr, usize>,
}

/// The connections open, counted.
struct Gate {
    counts: Mutex<Counts>,
    max: usize,
    per_address: Option<usize>,
}

/// A connection counted in the gate: given back when it drops.
struct Permit {
    gate: Arc<Gate>,
    key: IpAddr,
}

impl Gate {
    fn admit(gate: &Arc<Gate>, peer: IpAddr) -> Option<Permit> {
        let key = super::rate::address_key(peer);
        let mut counts = gate.counts.lock().unwrap_or_else(|e| e.into_inner());
        let mine = counts.by_address.get(&key).copied().unwrap_or(0);
        if counts.total >= gate.max || gate.per_address.is_some_and(|n| mine >= n) {
            return None;
        }
        counts.total += 1;
        *counts.by_address.entry(key).or_default() += 1;
        Some(Permit {
            gate: gate.clone(),
            key,
        })
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut counts = self.gate.counts.lock().unwrap_or_else(|e| e.into_inner());
        counts.total = counts.total.saturating_sub(1);
        if let Some(n) = counts.by_address.get_mut(&self.key) {
            *n -= 1;
            if *n == 0 {
                counts.by_address.remove(&self.key);
            }
        }
    }
}

/// A socket that notes when a byte last passed, and holds its place in the gate while it lives (upgraded too).
struct Watched<T> {
    inner: T,
    origin: Instant,
    /// Milliseconds since `origin` when a byte last passed.
    last: Arc<AtomicU64>,
    _permit: Permit,
}

impl<T> Watched<T> {
    fn touch(&self) {
        self.last
            .store(self.origin.elapsed().as_millis() as u64, Ordering::Relaxed);
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for Watched<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let polled = Pin::new(&mut self.inner).poll_read(cx, buf);
        if matches!(polled, Poll::Ready(Ok(()))) && buf.filled().len() > before {
            self.touch();
        }
        polled
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Watched<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let polled = Pin::new(&mut self.inner).poll_write(cx, buf);
        if matches!(polled, Poll::Ready(Ok(n)) if n > 0) {
            self.touch();
        }
        polled
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let polled = Pin::new(&mut self.inner).poll_write_vectored(cx, bufs);
        if matches!(polled, Poll::Ready(Ok(n)) if n > 0) {
            self.touch();
        }
        polled
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Resolves once nothing has passed on the socket for `idle`.
async fn silent(origin: Instant, last: Arc<AtomicU64>, idle: Duration) {
    loop {
        let quiet = origin
            .elapsed()
            .saturating_sub(Duration::from_millis(last.load(Ordering::Relaxed)));
        if quiet >= idle {
            return;
        }
        tokio::time::sleep(idle - quiet).await;
    }
}

/// The TLS of a certificate chain and its key, both PEM.
pub fn tls_config(cert: &Path, key: &Path) -> io::Result<Arc<rustls::ServerConfig>> {
    let invalid =
        |e: rustls_pki_types::pem::Error| io::Error::new(io::ErrorKind::InvalidData, e.to_string());
    let chain = CertificateDer::pem_file_iter(cert)
        .map_err(invalid)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(invalid)?;
    let key = PrivateKeyDer::from_pem_file(key).map_err(invalid)?;
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| io::Error::other(e.to_string()))?
    .with_no_client_auth()
    .with_single_cert(chain, key)
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

/// Serves `router` on `listener` until `shutdown` resolves, then lets the open connections end (10 s at most).
pub async fn serve(
    listener: TcpListener,
    router: Router,
    tls: Option<Arc<rustls::ServerConfig>>,
    bounds: Bounds,
    shutdown: impl Future<Output = ()> + Send,
) -> io::Result<()> {
    let acceptor = tls.map(tokio_rustls::TlsAcceptor::from);
    let (stop_tx, stop_rx) = watch::channel(false);
    let mut shutdown = std::pin::pin!(shutdown);
    let gate = Arc::new(Gate {
        counts: Mutex::new(Counts::default()),
        max: bounds.max_connections,
        per_address: bounds.per_address,
    });
    let idle = bounds.idle;
    loop {
        let (stream, peer) = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(pair) => pair,
                // Too many open files, a reset before accept: keep listening.
                Err(_) => { tokio::time::sleep(Duration::from_millis(50)).await; continue; }
            },
            _ = &mut shutdown => break,
        };
        // Past a bound, closed at once: dropped before a byte is read.
        let Some(permit) = Gate::admit(&gate, peer.ip()) else {
            if let Some(refused) = &bounds.on_refused {
                refused();
            }
            drop(stream);
            continue;
        };
        let _ = stream.set_nodelay(true);
        let origin = Instant::now();
        let last = Arc::new(AtomicU64::new(0));
        let stream = Watched {
            inner: stream,
            origin,
            last: last.clone(),
            _permit: permit,
        };
        let router = router.clone();
        let acceptor = acceptor.clone();
        let mut stop = stop_rx.clone();
        tokio::spawn(async move {
            let silent = silent(origin, last, idle);
            let service = hyper::service::service_fn(move |mut req: hyper::Request<Incoming>| {
                req.extensions_mut().insert(ConnectInfo(peer));
                router.clone().oneshot(req.map(axum::body::Body::new))
            });
            let mut builder = hyper::server::conn::http1::Builder::new();
            builder
                .timer(TokioTimer::new())
                .header_read_timeout(Duration::from_secs(15))
                .max_buf_size(64 << 10);
            match acceptor {
                None => {
                    let conn = builder
                        .serve_connection(TokioIo::new(stream), service)
                        .with_upgrades();
                    drive(conn, &mut stop, silent).await;
                }
                Some(acceptor) => {
                    let Ok(Ok(stream)) =
                        tokio::time::timeout(Duration::from_secs(15), acceptor.accept(stream))
                            .await
                    else {
                        return;
                    };
                    let conn = builder
                        .serve_connection(TokioIo::new(stream), service)
                        .with_upgrades();
                    drive(conn, &mut stop, silent).await;
                }
            }
        });
    }
    let _ = stop_tx.send(true);
    // The connections close gracefully; give them a moment.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while stop_tx.receiver_count() > 1 && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Ok(())
}

/// Drives a connection until it ends (or is upgraded: a WebSocket goes on by itself), is silent for its idle time
/// (dropped), or the server stops (gracefully).
async fn drive<C>(conn: C, stop: &mut watch::Receiver<bool>, silent: impl Future<Output = ()>)
where
    C: Future + GracefulConnection,
{
    let mut conn = std::pin::pin!(conn);
    tokio::select! {
        _ = conn.as_mut() => {}
        _ = silent => {}
        _ = stopped(stop) => {
            conn.as_mut().graceful();
            let _ = tokio::time::timeout(Duration::from_secs(10), conn).await;
        }
    }
}

async fn stopped(stop: &mut watch::Receiver<bool>) {
    loop {
        if *stop.borrow_and_update() {
            return;
        }
        if stop.changed().await.is_err() {
            // The listener is gone without a word: keep serving this connection.
            std::future::pending::<()>().await;
        }
    }
}

/// hyper's graceful shutdown, over the connection types `serve` makes.
pub trait GracefulConnection {
    fn graceful(self: std::pin::Pin<&mut Self>);
}

impl<I, S> GracefulConnection for hyper::server::conn::http1::UpgradeableConnection<I, S>
where
    I: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    S: hyper::service::HttpService<Incoming>,
    S::ResBody: 'static,
    <S::ResBody as hyper::body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn graceful(self: std::pin::Pin<&mut Self>) {
        hyper::server::conn::http1::UpgradeableConnection::graceful_shutdown(self)
    }
}

/// The address of the client: the peer's, or behind a trusted proxy the last of `X-Forwarded-For` (the one the
/// proxy saw: those before it come from the client and prove nothing).
pub fn client_address(
    peer: Option<SocketAddr>,
    forwarded: Option<&str>,
    trust_proxy: bool,
) -> Option<std::net::IpAddr> {
    if trust_proxy {
        if let Some(ip) = forwarded
            .and_then(|f| f.rsplit(',').next())
            .and_then(|last| last.trim().parse().ok())
        {
            return Some(ip);
        }
    }
    peer.map(|p| p.ip())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_is_the_peer_or_the_proxys_last_hop() {
        let peer: SocketAddr = "10.0.0.2:5000".parse().unwrap();
        let fwd = Some("1.1.1.1, 203.0.113.7");
        assert_eq!(
            client_address(Some(peer), fwd, false),
            Some("10.0.0.2".parse().unwrap())
        );
        assert_eq!(
            client_address(Some(peer), fwd, true),
            Some("203.0.113.7".parse().unwrap())
        );
        assert_eq!(
            client_address(Some(peer), Some("junk"), true),
            Some("10.0.0.2".parse().unwrap())
        );
    }
}
