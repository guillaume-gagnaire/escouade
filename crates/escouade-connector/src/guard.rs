//! The checks of each `open` (§ 4.4, R7), in their order:
//!
//! 1. the name and the port in the ceiling, after the shape; an address only when written as such ([`Ceiling::admit`]);
//! 2. the name resolved **once**, by the machine's DNS ([`Resolve`]);
//! 3. each address resolved checked **after** the resolution ([`forbidden`]): never the loopback, the link-local
//!    (`169.254.0.0/16`, whose `169.254.169.254` is a cloud's metadata), this machine's own addresses, the unspecified,
//!    the multicast or the broadcast, nor, when the ceiling has ranges, an address out of them;
//! 4. the address checked is the one joined, as such ([`join`]): no second resolution, so no DNS rebinding.
//!
//! The bounds (streams, opens a second) are the service's ([`crate::run`]).

use std::future::Future;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::time::Duration;

use escouade_connector_proto::{Refusal, RefusalCode};
use tokio::net::{TcpSocket, TcpStream};

use crate::ceiling::{canonical, show, Admission, Ceiling};

/// How long a resolution and a connection may take.
pub const RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// The machine's DNS, or a fake in the tests.
pub trait Resolve: Send + Sync {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<IpAddr>>> + Send + 'a>>;
}

/// The resolver of the system (`getaddrinfo`): the company's DNS, as the machine sees it.
pub struct SystemResolver;

impl Resolve for SystemResolver {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<IpAddr>>> + Send + 'a>> {
        Box::pin(async move {
            let addrs = tokio::net::lookup_host((host, port)).await?;
            Ok(addrs.map(|a| a.ip()).collect())
        })
    }
}

/// What the checks may be told, beyond the ceiling.
#[derive(Clone, Copy, Debug, Default)]
pub struct Guards {
    /// **For the tests alone**: the loopback allowed, so that hosts on 127.0.0.1 stand for a company's. Never set by
    /// `connector.toml` nor by the command line.
    pub loopback_for_tests: bool,
}

/// The well-known metadata addresses of clouds, said as such (all of them refused anyway).
const METADATA: [IpAddr; 3] = [
    IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
    // Alibaba Cloud.
    IpAddr::V4(Ipv4Addr::new(100, 100, 100, 200)),
    // AWS over IPv6.
    IpAddr::V6(Ipv6Addr::new(0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254)),
];

/// Why an address is never joined, whatever the ceiling: `None` when it may be.
pub fn forbidden(ip: IpAddr, guards: &Guards) -> Option<&'static str> {
    let ip = canonical(ip);
    if METADATA.contains(&ip) {
        return Some("a cloud's metadata");
    }
    let nat64 = |v6: &Ipv6Addr| {
        let s = v6.segments();
        (s[0] == 0x64 && s[1] == 0xff9b && s[2..6] == [0, 0, 0, 0])
            .then(|| Ipv4Addr::from(((s[6] as u32) << 16) | s[7] as u32))
    };
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_loopback() {
                return (!guards.loopback_for_tests).then_some("the loopback");
            }
            if v4.is_link_local() {
                return Some("a link-local address");
            }
            if v4.is_unspecified() || v4.octets()[0] == 0 {
                return Some("an unspecified address");
            }
            if v4.is_broadcast() || v4.is_multicast() {
                return Some("a broadcast or multicast address");
            }
            None
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() {
                return (!guards.loopback_for_tests).then_some("the loopback");
            }
            if (v6.segments()[0] & 0xffc0) == 0xfe80 {
                return Some("a link-local address");
            }
            if v6.is_unspecified() {
                return Some("an unspecified address");
            }
            if v6.is_multicast() {
                return Some("a multicast address");
            }
            if let Some(v4) = nat64(&v6) {
                return forbidden(IpAddr::V4(v4), guards);
            }
            if let Some(v4) = v6.to_ipv4() {
                // `::a.b.c.d`, the old compatible form.
                if v6.segments()[..6] == [0; 6] {
                    return forbidden(IpAddr::V4(v4), guards);
                }
            }
            None
        }
    }
}

/// Whether `ip` is one of this machine's own addresses: a socket bound to it succeeds only then. It is bound, never
/// listened on nor connected, and dropped at once (the connector listens on no port, § 4.6).
pub fn is_this_machine(ip: IpAddr) -> bool {
    let ip = canonical(ip);
    if ip.is_loopback() {
        return true;
    }
    let socket = match ip {
        IpAddr::V4(_) => TcpSocket::new_v4(),
        IpAddr::V6(_) => TcpSocket::new_v6(),
    };
    match socket {
        Ok(s) => s.bind(SocketAddr::new(ip, 0)).is_ok(),
        // Not even a socket: say it is, so that nothing is joined.
        Err(_) => true,
    }
}

/// Why an address resolved (or written) is refused, with the ceiling's ranges: `None` when it may be joined.
pub fn refuse_address(ip: IpAddr, ceiling: &Ceiling, guards: &Guards) -> Option<String> {
    let ip = canonical(ip);
    if let Some(why) = forbidden(ip, guards) {
        return Some(why.to_string());
    }
    if !(guards.loopback_for_tests && ip.is_loopback()) && is_this_machine(ip) {
        return Some("this connector's own machine".into());
    }
    if !ceiling.in_ranges(ip) {
        return Some("out of the ranges of the ceiling".into());
    }
    None
}

/// The checks passed: the addresses to join, in the resolver's order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Admitted {
    /// The host in shape.
    pub host: String,
    pub addrs: Vec<IpAddr>,
    /// The addresses resolved and refused, and why (for the journal).
    pub dropped: Vec<(IpAddr, String)>,
}

fn not_allowed(reason: String) -> Refusal {
    Refusal::new(RefusalCode::NotAllowed, reason)
}

/// Checks 1 to 3 for `host:port`: the addresses that may be joined, or the refusal.
pub async fn admit(
    host: &str,
    port: u16,
    ceiling: &Ceiling,
    resolver: &dyn Resolve,
    guards: &Guards,
) -> Result<Admitted, Refusal> {
    let (shaped, resolved) = match ceiling.admit(host, port) {
        Admission::Refused(why) => return Err(not_allowed(why)),
        Admission::Address(ip) => (ip.to_string(), vec![ip]),
        Admission::Name(name) => {
            let found = tokio::time::timeout(RESOLVE_TIMEOUT, resolver.resolve(&name, port))
                .await
                .map_err(|_| {
                    Refusal::new(
                        RefusalCode::Unresolved,
                        format!("{name} is not resolved in time by the connector's DNS"),
                    )
                })?
                .map_err(|e| {
                    Refusal::new(
                        RefusalCode::Unresolved,
                        format!("{name} is not resolved by the connector's DNS: {e}"),
                    )
                })?;
            if found.is_empty() {
                return Err(Refusal::new(
                    RefusalCode::Unresolved,
                    format!("{name} has no address in the connector's DNS"),
                ));
            }
            (name, found)
        }
    };
    let mut addrs = Vec::new();
    let mut dropped = Vec::new();
    for ip in resolved {
        let ip = canonical(ip);
        if addrs.contains(&ip) {
            continue;
        }
        match refuse_address(ip, ceiling, guards) {
            None => addrs.push(ip),
            Some(why) => dropped.push((ip, why)),
        }
    }
    if addrs.is_empty() {
        let said = dropped
            .iter()
            .map(|(ip, why)| format!("{ip}, {why}"))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(not_allowed(format!(
            "{} leads to an address the connector never joins ({said})",
            show(&shaped, port)
        )));
    }
    Ok(Admitted {
        host: shaped,
        addrs,
        dropped,
    })
}

/// Check 4: joins the first address checked that answers, as such (no resolution here).
pub async fn join(
    admitted: &Admitted,
    port: u16,
    timeout: Duration,
) -> Result<(TcpStream, SocketAddr), Refusal> {
    let mut last = String::new();
    for ip in &admitted.addrs {
        let addr = SocketAddr::new(*ip, port);
        match tokio::time::timeout(timeout, TcpStream::connect(addr)).await {
            Ok(Ok(tcp)) => {
                let _ = tcp.set_nodelay(true);
                return Ok((tcp, addr));
            }
            Ok(Err(e)) => last = format!("{addr}: {e}"),
            Err(_) => last = format!("{addr}: no answer in {} s", timeout.as_secs()),
        }
    }
    Err(Refusal::new(
        RefusalCode::Unreachable,
        format!(
            "{} does not answer from the connector ({last})",
            show(&admitted.host, port)
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn the_addresses_never_joined() {
        let g = Guards::default();
        for (a, why) in [
            ("127.0.0.1", "the loopback"),
            ("127.8.9.10", "the loopback"),
            ("::1", "the loopback"),
            ("::ffff:127.0.0.1", "the loopback"),
            ("169.254.169.254", "a cloud's metadata"),
            ("::ffff:169.254.169.254", "a cloud's metadata"),
            ("100.100.100.200", "a cloud's metadata"),
            ("fd00:ec2::254", "a cloud's metadata"),
            ("169.254.1.2", "a link-local address"),
            ("fe80::1", "a link-local address"),
            ("0.0.0.0", "an unspecified address"),
            ("0.1.2.3", "an unspecified address"),
            ("::", "an unspecified address"),
            ("255.255.255.255", "a broadcast or multicast address"),
            ("224.0.0.1", "a broadcast or multicast address"),
            ("ff02::1", "a multicast address"),
            ("64:ff9b::7f00:1", "the loopback"),
            ("64:ff9b::a9fe:a9fe", "a cloud's metadata"),
        ] {
            assert_eq!(forbidden(ip(a), &g), Some(why), "{a}");
        }
        for a in [
            "10.20.1.5",
            "192.168.1.1",
            "172.16.0.1",
            "fd00::5",
            "8.8.8.8",
        ] {
            assert_eq!(forbidden(ip(a), &g), None, "{a}");
        }
        let t = Guards {
            loopback_for_tests: true,
        };
        assert_eq!(forbidden(ip("127.0.0.1"), &t), None);
        // The option of the tests lifts the loopback alone.
        assert_eq!(
            forbidden(ip("169.254.169.254"), &t),
            Some("a cloud's metadata")
        );
    }

    #[tokio::test]
    async fn this_machine_is_known_by_a_bind() {
        assert!(is_this_machine(ip("127.0.0.1")));
        // TEST-NET-3: no machine holds it.
        assert!(!is_this_machine(ip("203.0.113.77")));
        // This machine's address towards the outside, without a packet sent (a UDP socket only connected).
        if let Ok(s) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if s.connect("192.0.2.1:9").is_ok() {
                if let Ok(local) = s.local_addr() {
                    if !local.ip().is_unspecified() {
                        assert!(is_this_machine(local.ip()), "{}", local.ip());
                    }
                }
            }
        }
    }
}
