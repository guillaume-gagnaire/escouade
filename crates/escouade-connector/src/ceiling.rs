//! The ceiling (§ 4.4, R4): what the company lets through, written in `connector.toml`, that the server cannot widen.
//!
//! - **A rule** is `host:port` (`gitlab.acme.internal:22`), `*.suffix:port` for every name under the suffix, not the
//!   suffix itself (`*.npm.acme.internal:443`), or an address written as such (`10.20.1.5:8443`, `[fd00::5]:22`):
//!   the only way an agent may name an address rather than a name.
//! - **A range** (`10.20.0.0/16`, `fd00:20::/32`), when there are any: every address joined, resolved or written,
//!   must be in one of them.
//! - **The shape**: a host is compared in lower case, without its final dot, an international name in `xn--`, an IPv6
//!   without its brackets, as the gate does (`escouade-gate::lists::shape`): both ends speak of the same host.

use std::fmt;
use std::net::IpAddr;

/// A host put in shape; `None` for what is no host.
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
        return Some(canonical(ip).to_string());
    }
    match url::Host::parse(h).ok()? {
        url::Host::Domain(d) => {
            let d = d.trim_end_matches('.');
            // A name of labels: no space, no slash, nothing a URL would have read past.
            if d.is_empty()
                || !d
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_')
            {
                return None;
            }
            Some(d.to_string())
        }
        url::Host::Ipv4(ip) => Some(ip.to_string()),
        url::Host::Ipv6(ip) => Some(canonical(IpAddr::V6(ip)).to_string()),
    }
}

/// An IPv4 mapped in IPv6 (`::ffff:127.0.0.1`) read as the IPv4 it is: the checks see the address joined.
pub fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        v4 => v4,
    }
}

/// A line of the ceiling.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Rule {
    /// A name and a port.
    Exact { host: String, port: u16 },
    /// Every name under `suffix` (not `suffix` itself), on `port`.
    Under { suffix: String, port: u16 },
    /// An address written as such, and a port.
    Address { ip: IpAddr, port: u16 },
}

/// Why a line of `connector.toml` is not read.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
#[error("« {line} » : {why}")]
pub struct BadRule {
    pub line: String,
    pub why: &'static str,
}

fn split_port(line: &str) -> Option<(&str, u16)> {
    let (host, port) = line.rsplit_once(':')?;
    if host.contains(':') && !(host.starts_with('[') && host.ends_with(']')) {
        return None;
    }
    let port = port.parse::<u16>().ok().filter(|p| *p != 0)?;
    Some((host, port))
}

impl Rule {
    pub fn parse(line: &str) -> Result<Rule, BadRule> {
        let text = line.trim();
        let bad = |why| BadRule {
            line: line.to_string(),
            why,
        };
        let (host, port) = split_port(text)
            .ok_or_else(|| bad("a host:port is expected, the port from 1 to 65535"))?;
        if let Some(suffix) = host.strip_prefix("*.") {
            let suffix = shape(suffix).ok_or_else(|| bad("not a name after « *. »"))?;
            if suffix.parse::<IpAddr>().is_ok() {
                return Err(bad("a pattern names names, not addresses"));
            }
            if !suffix.contains('.') {
                return Err(bad("a pattern is under a name of two labels at least"));
            }
            return Ok(Rule::Under { suffix, port });
        }
        if host.contains('*') {
            return Err(bad("a pattern starts with « *. »"));
        }
        let host = shape(host).ok_or_else(|| bad("not a host"))?;
        Ok(match host.parse::<IpAddr>() {
            Ok(ip) => Rule::Address { ip, port },
            Err(_) => Rule::Exact { host, port },
        })
    }

    /// Whether it lets `host` (in shape) and `port` through.
    pub fn matches(&self, host: &str, port: u16) -> bool {
        match self {
            Rule::Exact { host: h, port: p } => h == host && *p == port,
            Rule::Under { suffix, port: p } => {
                *p == port
                    && host.parse::<IpAddr>().is_err()
                    && host.len() > suffix.len() + 1
                    && host.ends_with(suffix.as_str())
                    && host.as_bytes()[host.len() - suffix.len() - 1] == b'.'
            }
            Rule::Address { ip, port: p } => {
                *p == port && host.parse::<IpAddr>().ok().map(canonical) == Some(*ip)
            }
        }
    }

    pub fn port(&self) -> u16 {
        match self {
            Rule::Exact { port, .. } | Rule::Under { port, .. } | Rule::Address { port, .. } => {
                *port
            }
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rule::Exact { host, port } => write!(f, "{host}:{port}"),
            Rule::Under { suffix, port } => write!(f, "*.{suffix}:{port}"),
            Rule::Address {
                ip: IpAddr::V6(ip),
                port,
            } => write!(f, "[{ip}]:{port}"),
            Rule::Address { ip, port } => write!(f, "{ip}:{port}"),
        }
    }
}

/// A range of addresses: `10.20.0.0/16`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Range {
    pub base: IpAddr,
    pub prefix: u8,
}

impl Range {
    pub fn parse(text: &str) -> Result<Range, BadRule> {
        let bad = |why| BadRule {
            line: text.to_string(),
            why,
        };
        let (ip, prefix) = text
            .trim()
            .split_once('/')
            .ok_or_else(|| bad("an address/length is expected (10.20.0.0/16)"))?;
        let ip = canonical(
            ip.trim()
                .parse::<IpAddr>()
                .map_err(|_| bad("not an address"))?,
        );
        let prefix: u8 = prefix
            .trim()
            .parse()
            .map_err(|_| bad("not a length after « / »"))?;
        let max = if ip.is_ipv4() { 32 } else { 128 };
        if prefix > max {
            return Err(bad("a length beyond the address"));
        }
        let range = Range { base: ip, prefix };
        if range.network() != ip {
            return Err(bad(
                "bits set after the length (write the network's first address)",
            ));
        }
        Ok(range)
    }

    fn bits(ip: IpAddr) -> (u128, u8) {
        match ip {
            IpAddr::V4(v4) => (u32::from(v4) as u128, 32),
            IpAddr::V6(v6) => (u128::from(v6), 128),
        }
    }

    fn mask(&self) -> u128 {
        let (width, all): (u32, u128) = if self.base.is_ipv4() {
            (32, u32::MAX as u128)
        } else {
            (128, u128::MAX)
        };
        let host_bits = width - self.prefix as u32;
        match host_bits {
            0 => all,
            b if b >= width => 0,
            b => all & !((1u128 << b) - 1),
        }
    }

    fn network(&self) -> IpAddr {
        let (b, _) = Range::bits(self.base);
        let n = b & self.mask();
        match self.base {
            IpAddr::V4(_) => IpAddr::V4((n as u32).into()),
            IpAddr::V6(_) => IpAddr::V6(n.into()),
        }
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = canonical(ip);
        if ip.is_ipv4() != self.base.is_ipv4() {
            return false;
        }
        let (a, _) = Range::bits(ip);
        let (b, _) = Range::bits(self.base);
        a & self.mask() == b & self.mask()
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.base, self.prefix)
    }
}

/// What the ceiling says of a host and a port asked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Admission {
    /// A name in the ceiling: to resolve.
    Name(String),
    /// An address written as such in the ceiling.
    Address(IpAddr),
    /// Refused, and why.
    Refused(String),
}

/// The whole ceiling: its rules and its ranges.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Ceiling {
    pub rules: Vec<Rule>,
    pub ranges: Vec<Range>,
}

impl Ceiling {
    /// The lines of `allow` and `ranges`; the first line not read is the error.
    pub fn parse(allow: &[String], ranges: &[String]) -> Result<Ceiling, BadRule> {
        let mut rules = Vec::new();
        for line in allow {
            let rule = Rule::parse(line)?;
            if !rules.contains(&rule) {
                rules.push(rule);
            }
        }
        let ranges = ranges
            .iter()
            .map(|r| Range::parse(r))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Ceiling { rules, ranges })
    }

    /// The first check of an `open` (§ 4.4): the name and the port in the ceiling after the shape; an address only if
    /// written as such.
    pub fn admit(&self, host: &str, port: u16) -> Admission {
        let Some(shaped) = shape(host) else {
            return Admission::Refused(format!("« {host} » is not a host"));
        };
        let shown = show(&shaped, port);
        match shaped.parse::<IpAddr>() {
            Ok(ip) => {
                if self.rules.iter().any(|r| r.matches(&shaped, port)) {
                    Admission::Address(ip)
                } else {
                    Admission::Refused(format!(
                        "{shown} is an address the ceiling of this connector does not write (it lets names through, resolved on its side)"
                    ))
                }
            }
            Err(_) => {
                if self.rules.iter().any(|r| r.matches(&shaped, port)) {
                    Admission::Name(shaped)
                } else {
                    Admission::Refused(format!("{shown} is not in the ceiling of this connector"))
                }
            }
        }
    }

    /// Whether `ip` is in the ranges (always, without ranges).
    pub fn in_ranges(&self, ip: IpAddr) -> bool {
        self.ranges.is_empty() || self.ranges.iter().any(|r| r.contains(ip))
    }

    /// The ceiling as the server is told it, at each connection (§ 4.4): its rules in shape, then its ranges
    /// (`10.20.0.0/16`, no port: shown, never a host).
    pub fn lines(&self) -> Vec<String> {
        self.rules
            .iter()
            .map(|r| r.to_string())
            .chain(self.ranges.iter().map(|r| r.to_string()))
            .collect()
    }
}

/// `host:port`, `[v6]:port`.
pub fn show(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ceiling(allow: &[&str], ranges: &[&str]) -> Ceiling {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        Ceiling::parse(&s(allow), &s(ranges)).unwrap()
    }

    #[test]
    fn hosts_are_compared_in_shape() {
        assert_eq!(
            shape("GitLab.Acme.Internal.").as_deref(),
            Some("gitlab.acme.internal")
        );
        assert_eq!(
            shape("bücher.example").as_deref(),
            Some("xn--bcher-kva.example")
        );
        assert_eq!(shape("[::FFFF:127.0.0.1]").as_deref(), Some("127.0.0.1"));
        assert_eq!(shape("[FD00::5]").as_deref(), Some("fd00::5"));
        assert_eq!(shape(""), None);
        assert_eq!(shape("a b"), None);
        assert_eq!(shape("a/b"), None);
        let c = ceiling(
            &["GITLAB.acme.internal.:22", "*.npm.acme.internal:443"],
            &[],
        );
        assert_eq!(
            c.admit("gitlab.ACME.internal.", 22),
            Admission::Name("gitlab.acme.internal".into())
        );
        assert!(matches!(
            c.admit("gitlab.acme.internal", 443),
            Admission::Refused(_)
        ));
        assert_eq!(
            c.admit("a.b.NPM.acme.internal", 443),
            Admission::Name("a.b.npm.acme.internal".into())
        );
        // The pattern is under the suffix, not the suffix itself, nor a name that only ends alike.
        assert!(matches!(
            c.admit("npm.acme.internal", 443),
            Admission::Refused(_)
        ));
        assert!(matches!(
            c.admit("evilnpm.acme.internal", 443),
            Admission::Refused(_)
        ));
        assert!(matches!(
            c.admit("x.npm.acme.internal", 80),
            Admission::Refused(_)
        ));
    }

    #[test]
    fn an_international_name_matches_its_xn_form() {
        let c = ceiling(&["bücher.acme.internal:443"], &[]);
        assert_eq!(c.rules[0].to_string(), "xn--bcher-kva.acme.internal:443");
        assert_eq!(
            c.admit("xn--bcher-kva.acme.internal", 443),
            Admission::Name("xn--bcher-kva.acme.internal".into())
        );
        assert_eq!(
            c.admit("BÜCHER.acme.internal", 443),
            Admission::Name("xn--bcher-kva.acme.internal".into())
        );
    }

    #[test]
    fn a_literal_address_passes_only_when_written() {
        let c = ceiling(
            &["gitlab.acme.internal:22", "10.20.1.5:8443", "[fd00::5]:22"],
            &[],
        );
        assert_eq!(
            c.admit("10.20.1.5", 8443),
            Admission::Address("10.20.1.5".parse().unwrap())
        );
        assert_eq!(
            c.admit("[FD00::5]", 22),
            Admission::Address("fd00::5".parse().unwrap())
        );
        assert_eq!(
            c.admit("::ffff:10.20.1.5", 8443),
            Admission::Address("10.20.1.5".parse().unwrap())
        );
        let Admission::Refused(why) = c.admit("10.20.1.6", 22) else {
            panic!()
        };
        assert!(
            why.contains("address the ceiling of this connector does not write"),
            "{why}"
        );
        assert!(matches!(c.admit("10.20.1.5", 22), Admission::Refused(_)));
        assert!(matches!(
            c.admit("169.254.169.254", 80),
            Admission::Refused(_)
        ));
    }

    #[test]
    fn bad_lines_are_said() {
        for line in [
            "gitlab",
            "gitlab:0",
            "gitlab:99999",
            "*.10.0.0.1:22",
            "*.internal:22",
            "a*b.acme:22",
            "::1:22",
        ] {
            assert!(Rule::parse(line).is_err(), "{line}");
        }
        for range in ["10.20.0.0", "10.20.0.1/16", "10.0.0.0/33", "x/8"] {
            assert!(Range::parse(range).is_err(), "{range}");
        }
    }

    #[test]
    fn ranges_hold_their_addresses() {
        let r = Range::parse("10.20.0.0/16").unwrap();
        assert!(r.contains("10.20.255.1".parse().unwrap()));
        assert!(r.contains("::ffff:10.20.0.9".parse().unwrap()));
        assert!(!r.contains("10.21.0.1".parse().unwrap()));
        assert!(!r.contains("fd00::1".parse().unwrap()));
        let all = Range::parse("0.0.0.0/0").unwrap();
        assert!(all.contains("8.8.8.8".parse().unwrap()));
        let one = Range::parse("10.20.1.5/32").unwrap();
        assert!(one.contains("10.20.1.5".parse().unwrap()));
        assert!(!one.contains("10.20.1.4".parse().unwrap()));
        let v6 = Range::parse("fd00:20::/32").unwrap();
        assert!(v6.contains("fd00:20:1::5".parse().unwrap()));
        assert!(!v6.contains("fd00:21::5".parse().unwrap()));
        let c = ceiling(&["a.acme.internal:1"], &["10.20.0.0/16"]);
        assert!(c.in_ranges("10.20.3.4".parse().unwrap()));
        assert!(!c.in_ranges("10.30.3.4".parse().unwrap()));
        assert!(Ceiling::default().in_ranges("10.30.3.4".parse().unwrap()));
        assert_eq!(c.lines(), ["a.acme.internal:1", "10.20.0.0/16"]);
    }
}
