//! `connector.toml` (§ 4.4, § 4.7), written by the company's IT, read again while the service runs ([`crate::run`]):
//!
//! ```toml
//! # The entry of the server's connectors (Space settings › Private network); `pair` gives it too.
//! server = "wss://escouade.example.com:8443/connector"
//! # The company's proxy out (http:// and Basic alone); left out: HTTPS_PROXY and NO_PROXY of the environment.
//! proxy = "http://user:password@proxy.acme.internal:3128"
//! # Authorities more, for the company's inspection of TLS (PEM).
//! authorities = "/etc/escouade-connector/acme-root.pem"
//!
//! # The ceiling: what the agents may reach, nothing else.
//! allow = [
//!   "gitlab.acme.internal:22",
//!   "*.npm.acme.internal:443",
//! ]
//! # Ranges every address joined must be in (optional).
//! ranges = ["10.20.0.0/16"]
//!
//! # Read alone for now: the update itself is RP-6's.
//! auto_update = false
//!
//! [limits]
//! max_streams = 256
//! opens_per_second = 32
//! idle_minutes = 10
//! ```
//!
//! An unknown key is an error (a typo does not pass for a rule left out).

use std::path::{Path, PathBuf};
use std::time::Duration;

use escouade_connector_proto::transport::Proxy;
use serde::Deserialize;

use crate::ceiling::Ceiling;

/// The hard bounds of a channel, beyond which `connector.toml` is not read.
pub const MAX_STREAMS: usize = 4096;
pub const MAX_OPENS_PER_SECOND: u32 = 1024;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct File {
    server: Option<String>,
    proxy: Option<String>,
    authorities: Option<PathBuf>,
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    ranges: Vec<String>,
    #[serde(default)]
    auto_update: bool,
    #[serde(default)]
    limits: FileLimits,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct FileLimits {
    max_streams: Option<usize>,
    opens_per_second: Option<u32>,
    idle_minutes: Option<u64>,
}

/// The bounds of the service (§ 4.4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bounds {
    pub max_streams: usize,
    pub opens_per_second: u32,
    pub idle: Duration,
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds {
            max_streams: 256,
            opens_per_second: 32,
            idle: Duration::from_secs(10 * 60),
        }
    }
}

/// `connector.toml`, read.
#[derive(Clone, Debug)]
pub struct Config {
    pub server: Option<String>,
    pub proxy: Option<Proxy>,
    pub authorities: Option<PathBuf>,
    pub ceiling: Ceiling,
    pub bounds: Bounds,
    pub auto_update: bool,
}

impl PartialEq for Config {
    fn eq(&self, other: &Config) -> bool {
        self.server == other.server
            && format!("{:?}", self.proxy) == format!("{:?}", other.proxy)
            && self.proxy.as_ref().and_then(|p| p.basic.clone())
                == other.proxy.as_ref().and_then(|p| p.basic.clone())
            && self.authorities == other.authorities
            && self.ceiling == other.ceiling
            && self.bounds == other.bounds
            && self.auto_update == other.auto_update
    }
}

impl Config {
    /// Whether the way to the server differs (another channel to open).
    pub fn way_differs(&self, other: &Config) -> bool {
        self.server != other.server
            || format!("{:?}", self.proxy) != format!("{:?}", other.proxy)
            || self.proxy.as_ref().and_then(|p| p.basic.clone())
                != other.proxy.as_ref().and_then(|p| p.basic.clone())
            || self.authorities != other.authorities
    }
}

/// Why `connector.toml` is not read. Never the proxy's password.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{path}: {why}")]
    Read { path: String, why: String },
    #[error("{path}: {why}")]
    Bad { path: String, why: String },
}

pub fn parse(text: &str, path: &Path) -> Result<Config, ConfigError> {
    let bad = |why: String| ConfigError::Bad {
        path: path.display().to_string(),
        why,
    };
    let file: File = toml::from_str(text).map_err(|e| {
        // TOML's message quotes the line: a password in it stays out of the journal.
        let why = e.message().to_string();
        bad(match e.span() {
            Some(span) => format!(
                "{why} (line {})",
                text[..span.start.min(text.len())].lines().count().max(1)
            ),
            None => why,
        })
    })?;
    if let Some(server) = &file.server {
        escouade_connector_proto::transport::parse_target(server)
            .map_err(|e| bad(format!("server: {e}")))?;
    }
    let proxy = match &file.proxy {
        Some(p) => Some(Proxy::parse(p).map_err(|_| {
            bad("proxy: http://[user:password@]host[:port] is expected (Basic alone)".into())
        })?),
        None => None,
    };
    let ceiling = Ceiling::parse(&file.allow, &file.ranges).map_err(|e| bad(e.to_string()))?;
    let d = Bounds::default();
    let bounds = Bounds {
        max_streams: file.limits.max_streams.unwrap_or(d.max_streams),
        opens_per_second: file.limits.opens_per_second.unwrap_or(d.opens_per_second),
        idle: file
            .limits
            .idle_minutes
            .map(|m| Duration::from_secs(m * 60))
            .unwrap_or(d.idle),
    };
    if bounds.max_streams == 0 || bounds.max_streams > MAX_STREAMS {
        return Err(bad(format!("limits.max_streams: from 1 to {MAX_STREAMS}")));
    }
    if bounds.opens_per_second == 0 || bounds.opens_per_second > MAX_OPENS_PER_SECOND {
        return Err(bad(format!(
            "limits.opens_per_second: from 1 to {MAX_OPENS_PER_SECOND}"
        )));
    }
    if bounds.idle.is_zero() || bounds.idle > Duration::from_secs(24 * 3600) {
        return Err(bad("limits.idle_minutes: from 1 to 1440".into()));
    }
    Ok(Config {
        server: file.server.map(|s| s.trim().to_string()),
        proxy,
        authorities: file.authorities,
        ceiling,
        bounds,
        auto_update: file.auto_update,
    })
}

/// Reads `connector.toml` at `path`, and its bytes (to tell a change).
pub fn load(path: &Path) -> Result<(Config, Vec<u8>), ConfigError> {
    let bytes = std::fs::read(path).map_err(|e| ConfigError::Read {
        path: path.display().to_string(),
        why: e.to_string(),
    })?;
    let text = String::from_utf8(bytes.clone()).map_err(|_| ConfigError::Bad {
        path: path.display().to_string(),
        why: "not UTF-8".into(),
    })?;
    Ok((parse(&text, path)?, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> Result<Config, ConfigError> {
        parse(text, Path::new("connector.toml"))
    }

    #[test]
    fn the_example_is_read() {
        let c = p(r#"
server = "wss://escouade.example.com:8443/connector"
proxy = "http://it:s3cret-pass@proxy.acme.internal:3128"
authorities = "/etc/escouade-connector/acme-root.pem"
allow = ["gitlab.acme.internal:22", "*.npm.acme.internal:443"]
ranges = ["10.20.0.0/16"]
auto_update = true
[limits]
max_streams = 64
opens_per_second = 8
idle_minutes = 2
"#)
        .unwrap();
        assert_eq!(
            c.server.as_deref(),
            Some("wss://escouade.example.com:8443/connector")
        );
        let proxy = c.proxy.as_ref().unwrap();
        assert_eq!(
            (proxy.host.as_str(), proxy.port),
            ("proxy.acme.internal", 3128)
        );
        assert_eq!(c.ceiling.rules.len(), 2);
        assert_eq!(c.ceiling.ranges.len(), 1);
        assert!(c.auto_update);
        assert_eq!(
            c.bounds,
            Bounds {
                max_streams: 64,
                opens_per_second: 8,
                idle: Duration::from_secs(120)
            }
        );
        assert!(!format!("{c:?}").contains("s3cret-pass"));
    }

    #[test]
    fn the_defaults_are_the_designs() {
        let c = p("allow = []").unwrap();
        assert_eq!(c.bounds, Bounds::default());
        assert_eq!(c.bounds.max_streams, 256);
        assert_eq!(c.bounds.opens_per_second, 32);
        assert_eq!(c.bounds.idle, Duration::from_secs(600));
        assert!(!c.auto_update);
        assert!(c.server.is_none() && c.proxy.is_none());
    }

    #[test]
    fn mistakes_are_said_without_the_password() {
        for (text, said) in [
            ("alow = []", "unknown field"),
            ("allow = [\"gitlab\"]", "gitlab"),
            ("ranges = [\"10.0.0.1/8\"]", "10.0.0.1/8"),
            ("server = \"https://x/connector\"", "server"),
            ("proxy = \"socks5://x:1\"", "proxy"),
            ("[limits]\nmax_streams = 0", "max_streams"),
            ("[limits]\nopens_per_second = 5000", "opens_per_second"),
            ("[limits]\nidle_minutes = 0", "idle_minutes"),
        ] {
            let e = p(text).unwrap_err().to_string();
            assert!(e.contains(said), "{text}: {e}");
        }
        let e = p("proxy = \"http://it:s3cret-pass@proxy:3128\"\nallow = 3")
            .unwrap_err()
            .to_string();
        assert!(!e.contains("s3cret-pass"), "{e}");
    }
}
