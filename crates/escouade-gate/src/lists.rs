//! The hosts, put in shape, the ceilings' patterns, and the list of each project (R4, R5): which hosts of a private
//! network a project reaches, each tied to its connector (R12). Empty by default: a project with no list reaches
//! nothing of a private network. Kept in `private-network.json` of the server's data folder; the gate reads it, the
//! user writes it (RP-4, RP-5), never an agent (no MCP tool reads nor writes it).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::Path;

/// The file of the lists, in the server's data folder.
pub const FILE: &str = "private-network.json";

/// A host put in shape: lower case, without its final dot, an international name in `xn--`, an IPv6 without its
/// brackets, an address in its usual spelling. `None` for what is no host.
pub fn shape(host: &str) -> Option<String> {
    let h = host.trim();
    let h = h
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .unwrap_or(h);
    let h = h.trim_end_matches('.');
    if h.is_empty() || h.len() > 253 {
        return None;
    }
    if let Ok(ip) = h.parse::<IpAddr>() {
        return Some(ip.to_string());
    }
    match url::Host::parse(h).ok()? {
        url::Host::Domain(d) => Some(d.trim_end_matches('.').to_string()),
        url::Host::Ipv4(ip) => Some(ip.to_string()),
        url::Host::Ipv6(ip) => Some(ip.to_string()),
    }
}

/// A host and a port, the host in shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Target {
    pub host: String,
    pub port: u16,
}

impl Target {
    /// `host` and `port`, the host in shape.
    pub fn new(host: &str, port: u16) -> Option<Self> {
        Some(Self {
            host: shape(host)?,
            port,
        })
    }

    /// `host:port` (`[::1]:22` for an IPv6), as a `CONNECT` names it.
    pub fn parse(authority: &str) -> Option<Self> {
        let (host, port) = authority.trim().rsplit_once(':')?;
        if host.contains(':') && !host.starts_with('[') {
            return None;
        }
        Self::new(host, port.parse().ok().filter(|p| *p != 0)?)
    }

    /// Whether its host is an address, not a name.
    pub fn is_address(&self) -> bool {
        self.host.parse::<IpAddr>().is_ok()
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.host.contains(':') {
            write!(f, "[{}]:{}", self.host, self.port)
        } else {
            write!(f, "{}:{}", self.host, self.port)
        }
    }
}

/// A line of a connector's ceiling (§ 4.4): `host:port`, or `*.suffix:port` for every name under the suffix (not the
/// suffix itself).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Exact(Target),
    Under { suffix: String, port: u16 },
}

impl Pattern {
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("*.") {
            let t = Target::parse(rest)?;
            if t.is_address() {
                return None;
            }
            return Some(Self::Under {
                suffix: t.host,
                port: t.port,
            });
        }
        Target::parse(line).map(Self::Exact)
    }

    /// Whether it names `host`, on any port: a name of its network.
    pub fn names(&self, host: &str) -> bool {
        match self {
            Self::Exact(e) => e.host == host,
            Self::Under { suffix, port } => {
                self.matches(&Target {
                    host: host.to_string(),
                    port: *port,
                }) && !suffix.is_empty()
            }
        }
    }

    pub fn matches(&self, t: &Target) -> bool {
        match self {
            Self::Exact(e) => e == t,
            Self::Under { suffix, port } => {
                *port == t.port
                    && !t.is_address()
                    && t.host.len() > suffix.len() + 1
                    && t.host.ends_with(suffix.as_str())
                    && t.host.as_bytes()[t.host.len() - suffix.len() - 1] == b'.'
            }
        }
    }
}

/// A host of a project's list, tied to the connector of its network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Allowed {
    pub host: String,
    pub port: u16,
    /// The id of its connector.
    pub connector: String,
}

/// A project's list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectList {
    #[serde(default)]
    pub hosts: Vec<Allowed>,
}

/// The lists of every project, as the file holds them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lists {
    #[serde(default)]
    pub projects: BTreeMap<String, ProjectList>,
}

impl Lists {
    /// The lists of `path`; none when it does not exist. A host that is none is left out, said in the log.
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e),
        };
        let read: Self = serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(read.shaped())
    }

    /// Every host in shape; what is no host left out.
    pub fn shaped(mut self) -> Self {
        for (project, list) in self.projects.iter_mut() {
            list.hosts.retain_mut(|a| match shape(&a.host) {
                Some(h) if a.port != 0 => {
                    a.host = h;
                    true
                }
                _ => {
                    log::warn!(
                        "gate: {:?}:{} of the project {project} is no host: left out",
                        a.host,
                        a.port
                    );
                    false
                }
            });
        }
        self
    }

    /// The entry of `project`'s list for `t`.
    pub fn find(&self, project: &str, t: &Target) -> Option<&Allowed> {
        self.projects
            .get(project)?
            .hosts
            .iter()
            .find(|a| a.host == t.host && a.port == t.port)
    }

    /// Whether `host` is named, on any port, in the list of any project: a name of a private network.
    pub fn names(&self, host: &str) -> bool {
        self.projects
            .values()
            .flat_map(|l| &l.hosts)
            .any(|a| a.host == host)
    }

    /// Whether `t` is in the list of any project: a host of a private network.
    pub fn anywhere(&self, t: &Target) -> bool {
        self.projects
            .values()
            .flat_map(|l| &l.hosts)
            .any(|a| a.host == t.host && a.port == t.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_put_in_shape() {
        assert_eq!(
            shape("GitLab.Acme.Internal.").as_deref(),
            Some("gitlab.acme.internal")
        );
        assert_eq!(shape("[::1]").as_deref(), Some("::1"));
        assert_eq!(
            shape("bücher.example").as_deref(),
            Some("xn--bcher-kva.example")
        );
        assert_eq!(shape("").as_deref(), None);
        assert_eq!(shape("a b").as_deref(), None);
        assert_eq!(
            Target::parse("[::1]:22").unwrap().to_string(),
            "[::1]:22".to_string()
        );
        assert_eq!(Target::parse("host:0"), None);
        assert_eq!(Target::parse("::1:22"), None);
    }

    #[test]
    fn a_pattern_takes_the_names_under_its_suffix_not_the_suffix() {
        let p = Pattern::parse("*.npm.acme.internal:443").unwrap();
        let t = |s: &str| Target::parse(s).unwrap();
        assert!(p.matches(&t("a.npm.acme.internal:443")));
        assert!(p.matches(&t("x.y.npm.acme.internal:443")));
        assert!(!p.matches(&t("npm.acme.internal:443")));
        assert!(!p.matches(&t("evilnpm.acme.internal:443")));
        assert!(!p.matches(&t("a.npm.acme.internal:80")));
        let e = Pattern::parse("GitLab.acme.internal:22").unwrap();
        assert!(e.matches(&t("gitlab.acme.internal:22")));
        assert!(Pattern::parse("*.10.0.0.1:22").is_none());
    }

    #[test]
    fn the_lists_are_read_in_shape_and_absent_is_empty() {
        let dir = std::env::temp_dir().join(format!("gate-lists-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE);
        assert_eq!(Lists::read(&path).unwrap(), Lists::default());
        std::fs::write(
            &path,
            r#"{"projects":{"p1":{"hosts":[{"host":"GitLab.Acme.Internal","port":22,"connector":"acme"},{"host":"a b","port":1,"connector":"acme"}]}}}"#,
        )
        .unwrap();
        let lists = Lists::read(&path).unwrap();
        let t = Target::parse("gitlab.acme.internal:22").unwrap();
        assert_eq!(lists.find("p1", &t).unwrap().connector, "acme");
        assert!(lists.find("p2", &t).is_none());
        assert!(lists.anywhere(&t));
        assert_eq!(lists.projects["p1"].hosts.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
