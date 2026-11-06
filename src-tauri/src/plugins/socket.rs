//! What the sockets of the plugins run over (`ws/open`, docs/PLUGINS.md §8.4.4): the settings' network, as reqwest follows
//! it. The proxy (an HTTP CONNECT tunnel, opened by hand, as reqwest opens one; who goes around it, `no_proxy`), and their
//! « certificats non vérifiés ».

use base64::Engine as _;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The network settings a WebSocket follows, as reqwest's client does: the proxy, who goes around it, and whether a
/// certificate is checked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Net {
    pub proxy: String,
    pub no_proxy: String,
    pub insecure: bool,
}

impl Net {
    pub fn of(settings: &crate::model::Settings) -> Self {
        Self {
            proxy: settings.proxy_url.trim().to_string(),
            no_proxy: settings.no_proxy.clone(),
            insecure: settings.insecure_tls,
        }
    }
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// What a WebSocket runs over: a TCP connection, TLS or not, through the proxy or not.
pub(crate) trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

pub(crate) type Ws = tokio_tungstenite::WebSocketStream<Box<dyn Io>>;

/// The proxy `host` goes through: the settings', unless `no_proxy` names it (a host, a domain and what is under it,
/// `*` for all).
pub(crate) fn proxy_for(net: &Net, host: &str) -> Option<reqwest::Url> {
    if net.proxy.is_empty() {
        return None;
    }
    let host = host.trim_matches(['[', ']']).to_lowercase();
    let around = net
        .no_proxy
        .split([',', ' ', ';'])
        .map(|e| e.trim().trim_start_matches('.').to_lowercase())
        .filter(|e| !e.is_empty())
        .any(|e| e == "*" || host == e || host.ends_with(&format!(".{e}")));
    if around {
        return None;
    }
    proxy_url(net)
}

/// The proxy of the settings, as an address.
pub(crate) fn proxy_url(net: &Net) -> Option<reqwest::Url> {
    if net.proxy.is_empty() {
        return None;
    }
    let raw = if net.proxy.contains("://") {
        net.proxy.clone()
    } else {
        format!("http://{}", net.proxy)
    };
    reqwest::Url::parse(&raw).ok()
}

/// A tunnel to `host:port` through the HTTP proxy `proxy` (CONNECT), its user and password sent when it has them.
pub(crate) async fn tunnel(
    proxy: &reqwest::Url,
    host: &str,
    port: u16,
) -> Result<tokio::net::TcpStream, String> {
    let at = (
        proxy.host_str().unwrap_or_default().to_string(),
        proxy.port_or_known_default().unwrap_or(80),
    );
    let mut s = tokio::net::TcpStream::connect((at.0.as_str(), at.1))
        .await
        .map_err(|e| format!("proxy {}:{}: {e}", at.0, at.1))?;
    let mut head = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n");
    if !proxy.username().is_empty() {
        let user = percent_decode(proxy.username());
        let pass = percent_decode(proxy.password().unwrap_or(""));
        head.push_str(&format!(
            "Proxy-Authorization: Basic {}\r\n",
            b64(format!("{user}:{pass}").as_bytes())
        ));
    }
    head.push_str("\r\n");
    s.write_all(head.as_bytes())
        .await
        .map_err(|e| format!("proxy: {e}"))?;
    let mut answer = Vec::new();
    let mut byte = [0u8; 1];
    while !answer.ends_with(b"\r\n\r\n") {
        let n = s.read(&mut byte).await.map_err(|e| format!("proxy: {e}"))?;
        if n == 0 || answer.len() > 16 * 1024 {
            return Err(tr!(
                "le proxy a fermé la connexion",
                "the proxy closed the connection"
            ));
        }
        answer.push(byte[0]);
    }
    let line = String::from_utf8_lossy(&answer);
    let line = line.lines().next().unwrap_or_default().to_string();
    let ok = line
        .split_whitespace()
        .nth(1)
        .is_some_and(|c| c.starts_with('2'));
    if !ok {
        return Err(tr!(
            "le proxy refuse le flux ({line})",
            "the proxy refuses the stream ({line})"
        ));
    }
    Ok(s)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// rustls with ring (as reqwest), the system's roots and Mozilla's, HTTP/1.1 only (a WebSocket's upgrade);
/// `insecure`: no certificate checked, as the settings may ask (a proxy that inspects TLS without its root installed).
pub(crate) fn tls_config(insecure: bool) -> Arc<rustls::ClientConfig> {
    static CHECKED: std::sync::OnceLock<Arc<rustls::ClientConfig>> = std::sync::OnceLock::new();
    static UNCHECKED: std::sync::OnceLock<Arc<rustls::ClientConfig>> = std::sync::OnceLock::new();
    let provider = || Arc::new(rustls::crypto::ring::default_provider());
    if insecure {
        return UNCHECKED
            .get_or_init(|| {
                let p = provider();
                let mut c = rustls::ClientConfig::builder_with_provider(p.clone())
                    .with_safe_default_protocol_versions()
                    .expect("ring's protocol versions")
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoCheck(p)))
                    .with_no_client_auth();
                c.alpn_protocols = vec![b"http/1.1".to_vec()];
                Arc::new(c)
            })
            .clone();
    }
    CHECKED
        .get_or_init(|| {
            let mut roots = rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let native = rustls_native_certs::load_native_certs();
            for e in &native.errors {
                log::warn!("voice: a root certificate of the system not read: {e}");
            }
            let (added, _) = roots.add_parsable_certificates(native.certs);
            log::info!("voice: {added} root certificates of the system for the live streams");
            let mut c = rustls::ClientConfig::builder_with_provider(provider())
                .with_safe_default_protocol_versions()
                .expect("ring's protocol versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
            c.alpn_protocols = vec![b"http/1.1".to_vec()];
            Arc::new(c)
        })
        .clone()
}

/// No certificate checked (« certificats non vérifiés » of the settings); the signatures still are.
#[derive(Debug)]
struct NoCheck(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for NoCheck {
    fn verify_server_cert(
        &self,
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_proxy_takes_hosts_domains_and_a_star() {
        let net = |no: &str| Net {
            proxy: "proxy.corp:8080".into(),
            no_proxy: no.into(),
            insecure: false,
        };
        assert_eq!(
            proxy_for(&net(""), "api.elevenlabs.io").unwrap().as_str(),
            "http://proxy.corp:8080/"
        );
        assert!(proxy_for(&net("elevenlabs.io"), "api.elevenlabs.io").is_none());
        assert!(proxy_for(&net(".elevenlabs.io"), "api.elevenlabs.io").is_none());
        assert!(proxy_for(&net("*"), "api.deepgram.com").is_none());
        assert!(proxy_for(&net("labs.io"), "api.elevenlabs.io").is_some());
        assert!(proxy_for(&Net::default(), "api.elevenlabs.io").is_none());
    }
}
