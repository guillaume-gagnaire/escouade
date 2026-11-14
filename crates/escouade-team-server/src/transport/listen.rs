//! Listening: HTTP/1.1 (WebSocket upgrades included), in clear behind a reverse proxy, or in TLS by rustls with a
//! given certificate. Each request carries the peer's address (`ConnectInfo<SocketAddr>`).

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::Router;
use hyper::body::Incoming;
use hyper_util::rt::{TokioIo, TokioTimer};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tower::ServiceExt as _;

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
    shutdown: impl Future<Output = ()> + Send,
) -> io::Result<()> {
    let acceptor = tls.map(tokio_rustls::TlsAcceptor::from);
    let (stop_tx, stop_rx) = watch::channel(false);
    let mut shutdown = std::pin::pin!(shutdown);
    loop {
        let (stream, peer) = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(pair) => pair,
                // Too many open files, a reset before accept: keep listening.
                Err(_) => { tokio::time::sleep(Duration::from_millis(50)).await; continue; }
            },
            _ = &mut shutdown => break,
        };
        let _ = stream.set_nodelay(true);
        let router = router.clone();
        let acceptor = acceptor.clone();
        let mut stop = stop_rx.clone();
        tokio::spawn(async move {
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
                    drive(conn, &mut stop).await;
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
                    drive(conn, &mut stop).await;
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

async fn drive<C>(conn: C, stop: &mut watch::Receiver<bool>)
where
    C: Future + GracefulConnection,
{
    let mut conn = std::pin::pin!(conn);
    tokio::select! {
        _ = conn.as_mut() => {}
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
