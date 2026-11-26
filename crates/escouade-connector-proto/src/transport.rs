//! The outer transport: a WebSocket over TLS (§ 4.2).
//!
//! - **The connector** ([`connect`]): `wss://<server>:<port C>/connector`, directly or through the company's HTTP
//!   proxy ([`Proxy`]: `CONNECT`, Basic authentication alone, `HTTPS_PROXY` and `NO_PROXY` by [`Proxy::from_env`]),
//!   its TLS checked against the system's authorities, Mozilla's, and a file of authorities more (a company that
//!   inspects TLS re-signs with its own). Who the server is, Noise says, not this certificate.
//! - **The server** ([`tls_acceptor`], [`accept_websocket`]): TLS with its certificate (or none behind a reverse proxy
//!   that holds it: Traefik, Caddy), then the WebSocket's upgrade on [`PATH`](crate::PATH) alone.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rustls_pki_types::pem::PemObject as _;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream, TlsAcceptor, TlsConnector};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::WebSocketStream;

use crate::channel::{AcceptError, ConnectError};
use crate::noise::MAX_MESSAGE;

/// The most bytes of the proxy's answer to `CONNECT`.
const MAX_PROXY_ANSWER: usize = 16 << 10;

/// The WebSocket's bounds: one message is one Noise message.
pub fn ws_config() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE))
}

/// An HTTP proxy for `CONNECT`, its user and password for Basic when it has them.
#[derive(Clone, PartialEq, Eq)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub basic: Option<(String, String)>,
}

impl fmt::Debug for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never the password.
        let user = self.basic.as_ref().map(|(u, _)| u.as_str());
        write!(f, "Proxy {{ {}:{}, user: {user:?} }}", self.host, self.port)
    }
}

impl Proxy {
    /// `http://[user:password@]host[:port]` (the port 80 by default). Another scheme is refused: an `https://` proxy,
    /// NTLM or Kerberos are not spoken (§ 4.10).
    pub fn parse(text: &str) -> Result<Proxy, ConnectError> {
        let bad = |why: &str| ConnectError::Net(format!("the proxy's address: {why}"));
        // `proxy:3128` alone is read as `http://proxy:3128`, as curl does.
        let text = text.trim();
        let full = if text.contains("://") {
            text.to_string()
        } else {
            format!("http://{text}")
        };
        let url = url::Url::parse(&full).map_err(|_| bad("not an address"))?;
        if url.scheme() != "http" {
            return Err(bad("an http:// proxy alone"));
        }
        let host = url.host_str().ok_or_else(|| bad("no host"))?;
        let host = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();
        let decode = |s: &str| {
            percent_encoding::percent_decode_str(s)
                .decode_utf8_lossy()
                .into_owned()
        };
        let basic = (!url.username().is_empty())
            .then(|| (decode(url.username()), decode(url.password().unwrap_or(""))));
        Ok(Proxy {
            host,
            port: url.port_or_known_default().unwrap_or(80),
            basic,
        })
    }

    /// The proxy of the environment for `host`: `HTTPS_PROXY` (or `https_proxy`), unless `NO_PROXY` (or `no_proxy`)
    /// names the host.
    pub fn from_env(host: &str) -> Option<Result<Proxy, ConnectError>> {
        let var = |names: &[&str]| {
            names
                .iter()
                .find_map(|n| std::env::var(n).ok().filter(|v| !v.trim().is_empty()))
        };
        let proxy = var(&["HTTPS_PROXY", "https_proxy"])?;
        if let Some(no) = var(&["NO_PROXY", "no_proxy"]) {
            if no_proxy_names(&no, host) {
                return None;
            }
        }
        Some(Proxy::parse(&proxy))
    }
}

/// Whether a `NO_PROXY` list names `host`: `*`, the host, or a domain above it (`.acme.fr` or `acme.fr`).
pub fn no_proxy_names(list: &str, host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    list.split(',').any(|entry| {
        let entry = entry.trim().to_ascii_lowercase();
        let entry = match entry.rsplit_once(':') {
            Some((name, port))
                if port.chars().all(|c| c.is_ascii_digit()) && !name.contains(':') =>
            {
                name.to_string()
            }
            _ => entry,
        };
        let entry = entry.trim_start_matches("*.").trim_start_matches('.');
        if entry == "*" {
            return true;
        }
        !entry.is_empty() && (host == entry || host.ends_with(&format!(".{entry}")))
    })
}

/// How the connector reaches its server.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// `wss://escouade.exemple.fr:8443/connector`.
    pub url: String,
    pub proxy: Option<Proxy>,
    /// A PEM file of authorities more (the company's, for its inspection of TLS).
    pub authorities: Option<PathBuf>,
    /// The system's authorities and Mozilla's (`false` in the tests: the file alone).
    pub system_roots: bool,
    /// The TCP connection, the proxy's tunnel, TLS and the upgrade, together.
    pub timeout: Duration,
}

impl ClientConfig {
    pub fn new(url: impl Into<String>) -> ClientConfig {
        ClientConfig {
            url: url.into(),
            proxy: None,
            authorities: None,
            system_roots: true,
            timeout: Duration::from_secs(30),
        }
    }
}

/// The parts of a `wss://` address.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub path: String,
}

pub fn parse_target(text: &str) -> Result<Target, ConnectError> {
    let bad = |why: &str| ConnectError::Net(format!("the server's address: {why}"));
    let url = url::Url::parse(text.trim()).map_err(|_| bad("not an address"))?;
    if url.scheme() != "wss" {
        return Err(bad("a wss:// address alone"));
    }
    let host = url.host_str().ok_or_else(|| bad("no host"))?;
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    let path = match url.path() {
        "" | "/" => crate::PATH.to_string(),
        p => p.to_string(),
    };
    Ok(Target {
        host,
        port: url.port_or_known_default().unwrap_or(443),
        path,
    })
}

/// The authorities a connector trusts for the outer TLS.
pub fn root_store(config: &ClientConfig) -> Result<rustls::RootCertStore, ConnectError> {
    let mut roots = rustls::RootCertStore::empty();
    if config.system_roots {
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        for cert in rustls_native_certs::load_native_certs().certs {
            let _ = roots.add(cert);
        }
    }
    if let Some(path) = &config.authorities {
        let bad = |why: String| {
            ConnectError::Net(format!("the authorities' file {}: {why}", path.display()))
        };
        let mut added = 0;
        for cert in CertificateDer::pem_file_iter(path).map_err(|e| bad(e.to_string()))? {
            let cert = cert.map_err(|e| bad(e.to_string()))?;
            roots.add(cert).map_err(|e| bad(e.to_string()))?;
            added += 1;
        }
        if added == 0 {
            return Err(bad("no certificate in it".into()));
        }
    }
    if roots.is_empty() {
        return Err(ConnectError::Net(
            "no authority to check the server's TLS".into(),
        ));
    }
    Ok(roots)
}

/// A tunnel to `host:port` through `proxy`, by `CONNECT`.
pub async fn tunnel(proxy: &Proxy, host: &str, port: u16) -> Result<TcpStream, ConnectError> {
    use base64::Engine as _;
    let net = |e: std::io::Error| {
        ConnectError::Net(format!("the proxy {}:{}: {e}", proxy.host, proxy.port))
    };
    let mut tcp = TcpStream::connect((proxy.host.as_str(), proxy.port))
        .await
        .map_err(net)?;
    let authority = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let mut head = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n");
    if let Some((user, password)) = &proxy.basic {
        let token = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
        head.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
    }
    head.push_str("\r\n");
    tcp.write_all(head.as_bytes()).await.map_err(net)?;
    // The answer's head byte by byte: nothing of the tunnel's own bytes is read past it.
    let mut answer = Vec::new();
    while !answer.ends_with(b"\r\n\r\n") {
        if answer.len() > MAX_PROXY_ANSWER {
            return Err(ConnectError::Net("the proxy's answer is too long".into()));
        }
        let mut b = [0u8; 1];
        if tcp.read(&mut b).await.map_err(net)? == 0 {
            return Err(ConnectError::Net("the proxy closed the tunnel".into()));
        }
        answer.push(b[0]);
    }
    let head = String::from_utf8_lossy(&answer);
    let status = head.split_whitespace().nth(1).unwrap_or("");
    if !status.starts_with('2') || status.len() != 3 {
        let why = match status {
            "407" => " (its authentication: Basic alone is spoken)".to_string(),
            _ => String::new(),
        };
        return Err(ConnectError::Net(format!(
            "the proxy refused the tunnel to {authority} ({status}){why}"
        )));
    }
    Ok(tcp)
}

/// The connector's WebSocket to its server.
pub async fn connect(
    config: &ClientConfig,
) -> Result<WebSocketStream<TlsStream<TcpStream>>, ConnectError> {
    tokio::time::timeout(config.timeout, connect_now(config))
        .await
        .map_err(|_| ConnectError::Net("no connection in time".into()))?
}

async fn connect_now(
    config: &ClientConfig,
) -> Result<WebSocketStream<TlsStream<TcpStream>>, ConnectError> {
    let target = parse_target(&config.url)?;
    let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| ConnectError::Net(e.to_string()))?
    .with_root_certificates(root_store(config)?)
    .with_no_client_auth();
    let tcp = match &config.proxy {
        Some(proxy) => tunnel(proxy, &target.host, target.port).await?,
        None => TcpStream::connect((target.host.as_str(), target.port))
            .await
            .map_err(|e| ConnectError::Net(format!("{}:{}: {e}", target.host, target.port)))?,
    };
    let _ = tcp.set_nodelay(true);
    let name = ServerName::try_from(target.host.clone())
        .map_err(|_| ConnectError::Net(format!("{} is not a name for TLS", target.host)))?;
    let tls = TlsConnector::from(Arc::new(tls))
        .connect(name, tcp)
        .await
        .map_err(|e| ConnectError::Net(format!("TLS with {}: {e}", target.host)))?;
    let (ws, _) =
        tokio_tungstenite::client_async_with_config(config.url.trim(), tls, Some(ws_config()))
            .await
            .map_err(|e| ConnectError::Net(format!("the WebSocket: {e}")))?;
    Ok(ws)
}

/// The server's TLS, with its chain and key in PEM.
pub fn tls_acceptor(chain_pem: &[u8], key_pem: &[u8]) -> Result<TlsAcceptor, String> {
    let chain = CertificateDer::pem_slice_iter(chain_pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("the certificate: {e}"))?;
    if chain.is_empty() {
        return Err("no certificate".into());
    }
    let key = PrivateKeyDer::from_pem_slice(key_pem).map_err(|e| format!("the key: {e}"))?;
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| e.to_string())?
    .with_no_client_auth()
    .with_single_cert(chain, key)
    .map_err(|e| e.to_string())?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// The WebSocket's upgrade on the server, on [`PATH`](crate::PATH) alone (another path: 404), within `timeout`.
pub async fn accept_websocket<S>(
    io: S,
    timeout: Duration,
) -> Result<WebSocketStream<S>, AcceptError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    #[allow(clippy::result_large_err)]
    let check = |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
        if req.uri().path() == crate::PATH {
            Ok(resp)
        } else {
            let mut refused = ErrorResponse::new(None);
            *refused.status_mut() = StatusCode::NOT_FOUND;
            Err(refused)
        }
    };
    tokio::time::timeout(
        timeout,
        tokio_tungstenite::accept_hdr_async_with_config(io, check, Some(ws_config())),
    )
    .await
    .map_err(|_| AcceptError::Timeout)?
    .map_err(|e| AcceptError::Network(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_proxy_is_read_as_curl_does() {
        let p = Proxy::parse("http://alice:p%40ss@proxy.acme.internal:3128").unwrap();
        assert_eq!(p.host, "proxy.acme.internal");
        assert_eq!(p.port, 3128);
        assert_eq!(p.basic, Some(("alice".into(), "p@ss".into())));
        assert!(!format!("{p:?}").contains("p@ss"), "never the password");
        let bare = Proxy::parse("proxy:8080").unwrap();
        assert_eq!(
            (bare.host.as_str(), bare.port, bare.basic),
            ("proxy", 8080, None)
        );
        assert_eq!(Proxy::parse("http://proxy").unwrap().port, 80);
        assert!(Proxy::parse("https://proxy:443").is_err());
        assert!(Proxy::parse("socks5://proxy:1080").is_err());
    }

    #[test]
    fn no_proxy_names_hosts_and_domains() {
        assert!(no_proxy_names(
            "localhost,.exemple.fr",
            "escouade.exemple.fr"
        ));
        assert!(no_proxy_names("exemple.fr", "escouade.exemple.fr"));
        assert!(no_proxy_names("*.exemple.fr", "escouade.exemple.fr"));
        assert!(no_proxy_names(
            "escouade.exemple.fr:8443",
            "Escouade.Exemple.fr."
        ));
        assert!(no_proxy_names("*", "anything"));
        assert!(!no_proxy_names("xemple.fr", "escouade.exemple.fr"));
        assert!(!no_proxy_names("", "escouade.exemple.fr"));
        assert!(!no_proxy_names("localhost", "escouade.exemple.fr"));
    }

    #[test]
    fn the_target_is_wss_alone() {
        let t = parse_target("wss://escouade.exemple.fr:8443/connector").unwrap();
        assert_eq!(
            t,
            Target {
                host: "escouade.exemple.fr".into(),
                port: 8443,
                path: "/connector".into()
            }
        );
        assert_eq!(parse_target("wss://e.fr").unwrap().port, 443);
        assert_eq!(parse_target("wss://e.fr").unwrap().path, "/connector");
        assert!(parse_target("ws://e.fr/connector").is_err());
        assert!(parse_target("https://e.fr/connector").is_err());
    }

    #[test]
    fn an_authorities_file_without_a_certificate_is_said() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("acme.pem");
        std::fs::write(&path, "not a certificate").unwrap();
        let mut config = ClientConfig::new("wss://e.fr/connector");
        config.system_roots = false;
        config.authorities = Some(path);
        assert!(root_store(&config).is_err());
        config.authorities = None;
        assert!(root_store(&config).is_err(), "no authority at all");
    }
}
