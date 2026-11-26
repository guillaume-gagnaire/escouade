//! Tailscale on the machine (ESC-567, M12 of `docs/MOBILE.md`, `docs/REMOTE.md` « Alternative sans WireGuard dans
//! l'app »): when it is there, the phone that has it too reaches the API at the machine's Tailscale address, with no
//! port to open in the box, CGNAT included. Escouade does not redo Tailscale: it finds its address and serves there.
//!
//! Found by its command (`tailscale ip -4`, then `status --json` for the machine's name), where it is installed or on
//! the `PATH`: the command alone says it then. Without the command, by the route to Tailscale's own address
//! (100.100.100.100), but only when the address it leaves from (100.64.0.0/10) is Tailscale's own interface's:
//! `tailscale0` (Linux), the adapter « Tailscale » (Windows), a `utun` whose 100.100.100.100 answers DNS as MagicDNS
//! does (macOS). An address of 100.64.0.0/10 is also an operator's CGNAT (a 4G key, a shared connection): on another
//! interface, it is no Tailscale, and the API is never served there (SR1-05 of the 2.3's security review).

use super::nat::shared;
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

/// Tailscale, as found on the machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// The machine's address in the tailnet.
    pub address: Ipv4Addr,
    /// Its name in the tailnet (MagicDNS), when the command says it.
    pub name: Option<String>,
}

/// How Tailscale is looked for.
#[derive(Debug, Clone)]
pub enum Probe {
    /// On the machine.
    Real,
    /// As given (the tests).
    Given(Option<Found>),
}

/// Whether `ip` is an address a tailnet gives (100.64.0.0/10).
pub fn tailnet(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => shared(v4),
        // fd7a:115c:a1e0::/48, Tailscale's IPv6 range.
        IpAddr::V6(v6) => {
            let s = v6.segments();
            s[0] == 0xfd7a && s[1] == 0x115c && s[2] == 0xa1e0
        }
    }
}

/// Tailscale's command, where it is.
fn command() -> Option<PathBuf> {
    let known: &[&str] = if cfg!(windows) {
        &[r"C:\Program Files\Tailscale\tailscale.exe"]
    } else if cfg!(target_os = "macos") {
        &[
            "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
            "/usr/local/bin/tailscale",
            "/opt/homebrew/bin/tailscale",
        ]
    } else {
        &["/usr/bin/tailscale", "/usr/local/bin/tailscale"]
    };
    known
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
        .or_else(|| which("tailscale"))
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    std::env::split_paths(&path)
        .map(|dir| dir.join(&exe))
        .find(|p| p.is_file())
}

async fn run(program: &PathBuf, args: &[&str]) -> Option<String> {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        // No console window flashes.
        cmd.creation_flags(0x0800_0000);
    }
    let out = tokio::time::timeout(Duration::from_secs(3), cmd.output())
        .await
        .ok()?
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Tailscale's address and name on this machine, if it runs.
pub async fn detect(probe: &Probe) -> Option<Found> {
    match probe {
        Probe::Given(found) => found.clone(),
        Probe::Real => {
            if let Some(program) = command() {
                let address = run(&program, &["ip", "-4"])
                    .await
                    .and_then(|out| out.lines().find_map(|l| l.trim().parse::<Ipv4Addr>().ok()))
                    .filter(|a| shared(*a));
                if let Some(address) = address {
                    let name = run(&program, &["status", "--json"])
                        .await
                        .and_then(|out| serde_json::from_str::<serde_json::Value>(&out).ok())
                        .and_then(|v| {
                            v["Self"]["DNSName"]
                                .as_str()
                                .map(|n| n.trim_end_matches('.').to_string())
                        })
                        .filter(|n| !n.is_empty());
                    return Some(Found { address, name });
                }
                return None;
            }
            // Without its command: the route to Tailscale's own address, from Tailscale's interface alone.
            let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
            socket.connect(SocketAddr::from((QUAD100, 53))).ok()?;
            let IpAddr::V4(route) = socket.local_addr().ok()?.ip() else {
                return None;
            };
            if !shared(route) {
                return None;
            }
            let interfaces = interfaces();
            let utun = interfaces
                .iter()
                .any(|i| i.address == route && kind(&i.name) == Kind::Utun);
            // MagicDNS asked only of a `utun` (another VPN may have one): nothing leaves otherwise.
            let magic_dns = utun && answers_dns(SocketAddr::from((QUAD100, 53))).await;
            by_interface(route, &interfaces, magic_dns).map(|address| Found {
                address,
                name: None,
            })
        }
    }
}

/// Tailscale's own address: its DNS (MagicDNS) answers there.
const QUAD100: Ipv4Addr = Ipv4Addr::new(100, 100, 100, 100);

/// An interface of the machine and its IPv4 address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub address: Ipv4Addr,
}

#[derive(Debug, PartialEq, Eq)]
enum Kind {
    /// Tailscale's own: `tailscale0`, the adapter « Tailscale ».
    Tailscale,
    /// A tunnel of macOS: Tailscale's, or another VPN's.
    Utun,
    Other,
}

fn kind(name: &str) -> Kind {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("tailscale") {
        Kind::Tailscale
    } else if lower.starts_with("utun") {
        Kind::Utun
    } else {
        Kind::Other
    }
}

/// The address the route to 100.100.100.100 leaves from, when it is Tailscale's: on Tailscale's interface, or on a
/// `utun` whose 100.100.100.100 answers DNS (`magic_dns`). An address of 100.64.0.0/10 on any other interface is an
/// operator's CGNAT: none.
pub fn by_interface(
    route: Ipv4Addr,
    interfaces: &[Interface],
    magic_dns: bool,
) -> Option<Ipv4Addr> {
    if !shared(route) {
        return None;
    }
    interfaces
        .iter()
        .filter(|i| i.address == route)
        .find(|i| match kind(&i.name) {
            Kind::Tailscale => true,
            Kind::Utun => magic_dns,
            Kind::Other => false,
        })
        .map(|i| i.address)
}

/// Whether a DNS server answers at `at` (a query for `ts.net`'s address, its answer read, whatever it says).
pub async fn answers_dns(at: SocketAddr) -> bool {
    let Ok(socket) = tokio::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await else {
        return false;
    };
    if socket.connect(at).await.is_err() {
        return false;
    }
    let mut id = [0u8; 2];
    if getrandom::fill(&mut id).is_err() {
        return false;
    }
    // A standard query, recursion desired, one question: ts.net, A, IN.
    let mut query = vec![id[0], id[1], 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
    query.extend_from_slice(b"\x02ts\x03net\x00\x00\x01\x00\x01");
    if socket.send(&query).await.is_err() {
        return false;
    }
    let mut buf = [0u8; 512];
    let read = tokio::time::timeout(Duration::from_secs(1), socket.recv(&mut buf)).await;
    // An answer to that query (its id, the answer bit).
    matches!(read, Ok(Ok(n)) if n >= 12 && buf[..2] == id && buf[2] & 0x80 != 0)
}

/// The machine's interfaces and their IPv4 addresses.
#[cfg(unix)]
pub fn interfaces() -> Vec<Interface> {
    let mut out = Vec::new();
    let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs fills `list` with a list the system owns until freeifaddrs; each entry is read while it lives.
    unsafe {
        if libc::getifaddrs(&mut list) != 0 {
            return out;
        }
        let mut at = list;
        while !at.is_null() {
            let entry = &*at;
            if !entry.ifa_addr.is_null() && i32::from((*entry.ifa_addr).sa_family) == libc::AF_INET
            {
                let sin = &*(entry.ifa_addr as *const libc::sockaddr_in);
                let name = std::ffi::CStr::from_ptr(entry.ifa_name)
                    .to_string_lossy()
                    .into_owned();
                out.push(Interface {
                    name,
                    address: Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)),
                });
            }
            at = entry.ifa_next;
        }
        libc::freeifaddrs(list);
    }
    out
}

/// The machine's adapters (their name as Windows shows it) and their IPv4 addresses.
#[cfg(windows)]
pub fn interfaces() -> Vec<Interface> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
        GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};
    const ERROR_BUFFER_OVERFLOW: u32 = 111;
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 16 * 1024;
    let mut out = Vec::new();
    for _ in 0..3 {
        // u64 for the alignment of IP_ADAPTER_ADDRESSES_LH.
        let mut buf = vec![0u64; (size as usize).div_ceil(8)];
        let first = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        // SAFETY: the buffer is `size` bytes at least, aligned; Windows writes a list into it that lives with it.
        let code = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_INET),
                flags,
                std::ptr::null(),
                first,
                &mut size,
            )
        };
        if code == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if code != 0 {
            return out;
        }
        let mut at = first as *const IP_ADAPTER_ADDRESSES_LH;
        // SAFETY: each adapter, its name and its addresses are in `buf`, read while it lives.
        unsafe {
            while !at.is_null() {
                let adapter = &*at;
                let name = wide(adapter.FriendlyName);
                let mut unicast = adapter.FirstUnicastAddress;
                while !unicast.is_null() {
                    let sockaddr = (*unicast).Address.lpSockaddr;
                    if !sockaddr.is_null() && (*sockaddr).sa_family == AF_INET {
                        let sin = &*(sockaddr as *const SOCKADDR_IN);
                        out.push(Interface {
                            name: name.clone(),
                            address: Ipv4Addr::from(u32::from_be(sin.sin_addr.S_un.S_addr)),
                        });
                    }
                    unicast = (*unicast).Next;
                }
                at = adapter.Next;
            }
        }
        return out;
    }
    out
}

/// A string of Windows (UTF-16, ended by a zero).
#[cfg(windows)]
unsafe fn wide(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0;
    // SAFETY: the string ends with a zero (the caller's).
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(name: &str, a: [u8; 4]) -> Interface {
        Interface {
            name: name.into(),
            address: Ipv4Addr::from(a),
        }
    }

    #[test]
    fn an_operators_cgnat_address_without_tailscales_interface_is_no_tailscale() {
        // The machine's own interfaces are read (its loopback at least), so the check below runs on real names.
        assert!(
            interfaces()
                .iter()
                .any(|i| i.address == Ipv4Addr::LOCALHOST),
            "{:?}",
            interfaces()
        );
        // A 4G key, a shared connection, a host in CGNAT: their address is in 100.64.0.0/10 too.
        for (iface, route) in [
            ("wwan0", [100, 72, 13, 9]),
            ("Ethernet", [100, 64, 0, 1]),
            ("Cellular", [100, 127, 255, 254]),
            ("en0", [100, 100, 1, 2]),
        ] {
            let list = [at("lo", [127, 0, 0, 1]), at(iface, route)];
            assert_eq!(
                by_interface(Ipv4Addr::from(route), &list, true),
                None,
                "{iface}"
            );
        }
        // No interface holds the address at all.
        assert_eq!(by_interface(Ipv4Addr::new(100, 72, 13, 9), &[], true), None);
        // Tailscale's interface is there, but the route leaves from the operator's address.
        let list = [
            at("tailscale0", [100, 101, 102, 103]),
            at("wwan0", [100, 72, 13, 9]),
        ];
        assert_eq!(
            by_interface(Ipv4Addr::new(100, 72, 13, 9), &list, true),
            None
        );
        // Out of 100.64.0.0/10, never.
        let list = [at("tailscale0", [192, 168, 1, 20])];
        assert_eq!(
            by_interface(Ipv4Addr::new(192, 168, 1, 20), &list, true),
            None
        );
    }

    #[test]
    fn tailscales_interface_says_it() {
        let route = Ipv4Addr::new(100, 101, 102, 103);
        // Linux and Windows: its own interface, MagicDNS not asked.
        for name in ["tailscale0", "Tailscale"] {
            let list = [at("eth0", [192, 168, 1, 20]), at(name, route.octets())];
            assert_eq!(by_interface(route, &list, false), Some(route), "{name}");
        }
        // macOS: a `utun` only when 100.100.100.100 answers as MagicDNS does (another VPN has a `utun` too).
        let list = [at("en0", [192, 168, 1, 20]), at("utun4", route.octets())];
        assert_eq!(by_interface(route, &list, true), Some(route));
        assert_eq!(by_interface(route, &list, false), None);
    }

    #[tokio::test]
    async fn magic_dns_is_an_answer_to_the_query_sent() {
        // A fake DNS on the loopback: it answers with the query's id and the answer bit.
        let server = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = server.local_addr().unwrap();
        std::thread::spawn(move || {
            let mut buf = [0u8; 512];
            if let Ok((n, from)) = server.recv_from(&mut buf) {
                let mut out = buf[..n].to_vec();
                out[2] |= 0x80;
                let _ = server.send_to(&out, from);
            }
        });
        assert!(answers_dns(addr).await);
        // Nothing listens: no MagicDNS.
        let silent = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        assert!(!answers_dns(silent.local_addr().unwrap()).await);
    }
}
