//! What the sockets of the plugins run over (`ws/open`, docs/PLUGINS-DESIGN.md §8.4.4): the settings' network, as reqwest follows
//! it. The proxy (an HTTP CONNECT tunnel, opened by hand, as reqwest opens one; who goes around it, `no_proxy`), and their
//! « certificats non vérifiés ».

use base64::Engine as _;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The network settings a WebSocket follows, as reqwest's client does (`crate::tls`).
pub use crate::tls::Net;
pub(crate) use crate::tls::{proxy_for, proxy_url};

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// What a WebSocket runs over: a TCP connection, TLS or not, through the proxy or not.
pub(crate) trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

pub(crate) type Ws = tokio_tungstenite::WebSocketStream<Box<dyn Io>>;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_proxy_takes_hosts_domains_and_a_star() {
        let net = |no: &str| Net {
            proxy: "proxy.corp:8080".into(),
            no_proxy: no.into(),
            ..Net::default()
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
