//! The way out for what is not of a private network (the models' APIs, public npm, GitHub): straight out, or through
//! the proxy set in the space (Réglages › Avancé › Réseau), which then stands behind the gate. Its `NO_PROXY` goes
//! straight out, as before.

use crate::connectors::Flow;
use crate::journal::Route;
use crate::lists::Target;
use crate::pipe::{read_head, Prefixed};
use crate::reason::Reason;
use base64::Engine as _;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// The time to open a connection out.
pub const CONNECT_WITHIN: Duration = Duration::from_secs(20);

/// The space's proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upstream {
    pub host: String,
    pub port: u16,
    /// Its `Proxy-Authorization`, when its address has a user's name.
    pub auth: Option<String>,
}

/// How the gate goes out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outside {
    pub upstream: Option<Upstream>,
    /// The names that go straight out all the same (its `NO_PROXY`), in lower case.
    pub no_proxy: Vec<String>,
    /// A proxy is set that the gate cannot speak to (`https://`, `socks5://`): nothing goes out but its `NO_PROXY`,
    /// rather than around it (« Ne jamais sortir sans le proxy »).
    pub unspoken: Option<String>,
}

/// A `NO_PROXY` list, split.
pub fn no_proxy_of(list: &str) -> Vec<String> {
    list.split(',')
        .map(|h| h.trim().trim_start_matches('.').to_ascii_lowercase())
        .map(|h| match h.rsplit_once(':') {
            Some((name, port)) if port.parse::<u16>().is_ok() && !name.contains(':') => {
                name.to_string()
            }
            _ => h,
        })
        .filter(|h| !h.is_empty())
        .collect()
}

/// Whether `host` is in `list` (`no_proxy_of`): itself, a name under it, or `*`.
pub fn bypassed(list: &[String], host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    list.iter().any(|n| {
        n == "*"
            || host == *n
            || (host.len() > n.len()
                && host.ends_with(n.as_str())
                && host.as_bytes()[host.len() - n.len() - 1] == b'.')
    })
}

impl Outside {
    /// The proxy of the settings (`url`, its password in it, and its `NO_PROXY`); straight out without one.
    pub fn of_proxy(url: &str, no_proxy: &str) -> Self {
        let url = url.trim();
        let no_proxy = no_proxy_of(no_proxy);
        if url.is_empty() {
            return Self {
                no_proxy,
                ..Self::default()
            };
        }
        let spelled = if url.contains("://") {
            url.to_string()
        } else {
            format!("http://{url}")
        };
        let parsed = url::Url::parse(&spelled).ok();
        let Some(u) = parsed.filter(|u| u.scheme() == "http" && u.host_str().is_some()) else {
            let scheme = spelled.split("://").next().unwrap_or("").to_string();
            return Self {
                no_proxy,
                unspoken: Some(scheme),
                ..Self::default()
            };
        };
        let decode = |s: &str| {
            url::form_urlencoded::parse(format!("x={s}").as_bytes())
                .next()
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default()
        };
        let auth = (!u.username().is_empty()).then(|| {
            let pair = format!(
                "{}:{}",
                decode(u.username()),
                decode(u.password().unwrap_or(""))
            );
            format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(pair)
            )
        });
        let host = u.host_str().unwrap_or_default();
        let host = host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(host)
            .to_string();
        Self {
            upstream: Some(Upstream {
                host,
                port: u.port_or_known_default().unwrap_or(80),
                auth,
            }),
            no_proxy,
            unspoken: None,
        }
    }

    /// The space's proxy for `t`, unless its `NO_PROXY` sends it straight out; or why nothing goes out.
    fn upstream_for(&self, t: &Target) -> Result<Option<&Upstream>, Reason> {
        if bypassed(&self.no_proxy, &t.host) {
            return Ok(None);
        }
        if let Some(scheme) = &self.unspoken {
            return Err(Reason::Unreachable {
                target: t.to_string(),
                why: format!("the space's proxy ({scheme}://) is not one the gate speaks to"),
            });
        }
        Ok(self.upstream.as_ref())
    }

    /// A tunnel to `t`: straight, or by a `CONNECT` to the space's proxy.
    pub async fn tunnel(&self, t: &Target) -> Result<(Flow, Route), Reason> {
        match self.upstream_for(t)? {
            None => Ok((Box::new(direct(t).await?), Route::Direct)),
            Some(up) => {
                let mut s = to_upstream(up, t).await?;
                let mut ask = format!("CONNECT {t} HTTP/1.1\r\nHost: {t}\r\n");
                if let Some(a) = &up.auth {
                    ask.push_str(&format!("Proxy-Authorization: {a}\r\n"));
                }
                ask.push_str("\r\n");
                let broken = |e: std::io::Error| Reason::Unreachable {
                    target: t.to_string(),
                    why: format!("the space's proxy: {e}"),
                };
                s.write_all(ask.as_bytes()).await.map_err(broken)?;
                let (head, rest) = read_head(&mut s).await.map_err(broken)?;
                let mut headers = [httparse::EMPTY_HEADER; 64];
                let mut answer = httparse::Response::new(&mut headers);
                let status = match answer.parse(&head) {
                    Ok(httparse::Status::Complete(_)) => answer.code.unwrap_or(0),
                    _ => 0,
                };
                if !(200..300).contains(&status) {
                    return Err(Reason::UpstreamRefused {
                        target: t.to_string(),
                        status,
                    });
                }
                Ok((Box::new(Prefixed::new(rest, s)), Route::Proxy))
            }
        }
    }

    /// A connection for a request in plain HTTP to `t`: to `t` itself, or to the space's proxy, which then takes the
    /// request in its absolute form with the authorization given back.
    pub async fn plain(&self, t: &Target) -> Result<(Flow, Route, Option<Option<String>>), Reason> {
        match self.upstream_for(t)? {
            None => Ok((Box::new(direct(t).await?), Route::Direct, None)),
            Some(up) => Ok((
                Box::new(to_upstream(up, t).await?),
                Route::Proxy,
                Some(up.auth.clone()),
            )),
        }
    }
}

async fn direct(t: &Target) -> Result<TcpStream, Reason> {
    let broken = |why: String| Reason::Unreachable {
        target: t.to_string(),
        why,
    };
    let s = tokio::time::timeout(
        CONNECT_WITHIN,
        TcpStream::connect((t.host.as_str(), t.port)),
    )
    .await
    .map_err(|_| broken("timed out".into()))?
    .map_err(|e| broken(e.to_string()))?;
    let _ = s.set_nodelay(true);
    Ok(s)
}

async fn to_upstream(up: &Upstream, t: &Target) -> Result<TcpStream, Reason> {
    let broken = |why: String| Reason::Unreachable {
        target: t.to_string(),
        why: format!("the space's proxy {}:{}: {why}", up.host, up.port),
    };
    let s = tokio::time::timeout(
        CONNECT_WITHIN,
        TcpStream::connect((up.host.as_str(), up.port)),
    )
    .await
    .map_err(|_| broken("timed out".into()))?
    .map_err(|e| broken(e.to_string()))?;
    let _ = s.set_nodelay(true);
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_proxy_is_read_with_its_password_and_its_no_proxy() {
        let o = Outside::of_proxy(
            "http://bob:p%40ss@proxy.corp:3128",
            "localhost, .corp.lan,10.0.0.1:80",
        );
        let up = o.upstream.clone().unwrap();
        assert_eq!((up.host.as_str(), up.port), ("proxy.corp", 3128));
        let pair = base64::engine::general_purpose::STANDARD.encode("bob:p@ss");
        assert_eq!(up.auth.as_deref(), Some(format!("Basic {pair}").as_str()));
        assert!(bypassed(&o.no_proxy, "localhost"));
        assert!(bypassed(&o.no_proxy, "a.corp.lan"));
        assert!(bypassed(&o.no_proxy, "corp.lan"));
        assert!(bypassed(&o.no_proxy, "10.0.0.1"));
        assert!(!bypassed(&o.no_proxy, "evilcorp.lan"));
        assert_eq!(Outside::of_proxy("", "").upstream, None);
        assert_eq!(
            Outside::of_proxy("proxy:8080", "").upstream.unwrap().port,
            8080
        );
        let s = Outside::of_proxy("socks5://p:1080", "");
        assert_eq!(s.unspoken.as_deref(), Some("socks5"));
    }
}
