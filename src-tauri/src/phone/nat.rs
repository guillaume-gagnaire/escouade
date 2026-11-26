//! The phone's UDP port seen from outside (ESC-567, `docs/REMOTE.md`, « Configuration, pour l'utilisateur » and
//! « Points durs (sur le PC) »): the machine's addresses, the port opened in the box by UPnP or NAT-PMP (its lease
//! renewed, removed when the access goes off), the box's own address on the Internet, the public address as seen from
//! outside (a STUN server: an IP and a port, nothing else said), and from them a CGNAT or a double NAT, or a machine
//! that has its public address itself (a VPS).
//!
//! Nothing here opens anything on its own: [`probe`] is told whether to open, and where to look ([`Probes`]). The
//! tests give a fake gateway and a fake STUN server on the loopback; they never reach the user's box.

use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use tokio::net::UdpSocket;

/// The NAT-PMP port of a gateway (RFC 6886).
pub const NATPMP_PORT: u16 = 5351;
/// The lease asked of the box, renewed at half of it.
pub const LEASE: u32 = 3600;
/// What the box shows of the mapping.
const DESCRIPTION: &str = "Escouade (phone)";

/// Where the probes look.
#[derive(Debug, Clone)]
pub struct Probes {
    /// The SSDP address UPnP's search goes to (`239.255.255.250:1900`); none: no UPnP.
    pub upnp: Option<SocketAddr>,
    /// The gateway NAT-PMP asks; none: the UPnP gateway's, else the `.1` of the machine's network.
    pub natpmp: Option<SocketAddr>,
    /// Whether NAT-PMP is tried at all.
    pub natpmp_on: bool,
    /// The STUN servers, in order (`host:port`); empty: the public address is not looked for.
    pub stun: Vec<String>,
    /// How long a probe waits for an answer.
    pub wait: Duration,
    /// The machine's address on its network, given (the tests); none: the system's route says it.
    pub local: Option<IpAddr>,
}

impl Probes {
    /// The real ones: the box by UPnP and NAT-PMP, the public address by two STUN servers.
    pub fn real() -> Self {
        Self {
            upnp: Some(SocketAddr::from(([239, 255, 255, 250], 1900))),
            natpmp: None,
            natpmp_on: true,
            stun: vec![
                "stun.cloudflare.com:3478".into(),
                "stun.l.google.com:19302".into(),
            ],
            wait: Duration::from_secs(3),
            local: None,
        }
    }

    /// None at all: nothing leaves the machine (the e2e tests, `ESCOUADE_PHONE_NET=off` in a debug build).
    pub fn off() -> Self {
        Self {
            upnp: None,
            natpmp: None,
            natpmp_on: false,
            stun: Vec::new(),
            wait: Duration::from_millis(200),
            local: None,
        }
    }

    pub fn is_off(&self) -> bool {
        self.upnp.is_none() && !self.natpmp_on && self.stun.is_empty()
    }
}

/// How the port was opened in the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Opener {
    Upnp,
    Natpmp,
}

/// The port opened in the box.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub by: Opener,
    pub external_port: u16,
    /// Seconds the box gives it (0: until removed).
    pub lease: u32,
    /// The box that mapped it: NAT-PMP removes it there.
    #[serde(skip)]
    pub at: Option<SocketAddr>,
}

/// What the network says, at one look.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetState {
    /// The machine's IPv4 address on its network: where the box sends the port.
    pub local: Option<IpAddr>,
    /// A global IPv6 address of the machine, reachable without a redirection when the box's firewall lets it.
    pub ipv6: Option<Ipv6Addr>,
    /// The box that answered (UPnP or NAT-PMP).
    pub gateway: Option<IpAddr>,
    /// The port opened in the box, by UPnP or NAT-PMP.
    pub opened: Option<Opened>,
    /// Why it could not be opened, as the box said it (or that none answered).
    pub open_error: Option<String>,
    /// The box's address on the Internet, as it says it.
    pub wan: Option<IpAddr>,
    /// The address seen from outside (STUN).
    pub public: Option<IpAddr>,
    /// The box's address is not the public one, or is shared (100.64.0.0/10, a private one): a redirection does
    /// nothing from outside.
    pub cgnat: bool,
    /// The machine has the public address itself (a VPS): no box, its own firewall.
    pub direct: bool,
    /// Whether anything was looked at (false: the probes are off).
    pub probed: bool,
}

/// Whether `ip` is in 100.64.0.0/10, the operators' shared space (RFC 6598), Tailscale's too.
pub fn shared(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (o[1] & 0xc0) == 64
}

/// Whether `ip` is no address of the Internet: private, shared, loopback, link-local.
fn not_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || shared(v4)
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
        }
        IpAddr::V6(v6) => !global_v6(v6),
    }
}

/// Whether `ip` is an address of the Internet.
pub fn is_public(ip: IpAddr) -> bool {
    !not_public(ip)
}

fn global_v6(ip: Ipv6Addr) -> bool {
    // 2000::/3, the global unicast addresses.
    (ip.segments()[0] & 0xe000) == 0x2000
}

/// The address the system would send from toward `to` (nothing is sent: a UDP socket only connects).
fn route_from(to: SocketAddr) -> Option<IpAddr> {
    let bind: SocketAddr = match to {
        SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
        SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
    };
    let socket = std::net::UdpSocket::bind(bind).ok()?;
    socket.connect(to).ok()?;
    Some(socket.local_addr().ok()?.ip())
}

/// The machine's IPv4 address on its network (the route to the Internet's).
pub fn local_v4() -> Option<IpAddr> {
    // TEST-NET-1: never reached, only routed.
    route_from(SocketAddr::from(([192, 0, 2, 1], 9))).filter(|ip| !ip.is_unspecified())
}

/// A global IPv6 address of the machine, if it has one.
pub fn local_v6() -> Option<Ipv6Addr> {
    match route_from(SocketAddr::from((
        "2001:db8::1".parse::<Ipv6Addr>().ok()?,
        9,
    )))? {
        IpAddr::V6(v6) if global_v6(v6) => Some(v6),
        _ => None,
    }
}

/// One look: the addresses, the port opened in the box when `open` (or kept open: a mapping is renewed by asking
/// again), the public address. `before` is the last look's mapping, renewed where it was made.
pub async fn probe(probes: &Probes, port: u16, open: bool) -> NetState {
    let mut state = NetState {
        local: probes.local.or_else(local_v4),
        ipv6: if probes.local.is_some() {
            None
        } else {
            local_v6()
        },
        probed: !probes.is_off(),
        ..NetState::default()
    };
    if probes.is_off() {
        return state;
    }
    let local = state.local;
    let mut errors: Vec<String> = Vec::new();
    // UPnP first: most boxes speak it.
    let mut upnp_gateway: Option<IpAddr> = None;
    if let (Some(ssdp), Some(IpAddr::V4(local))) = (probes.upnp, local) {
        match upnp(ssdp, probes.wait, local, port, open).await {
            Ok((gateway, wan, opened)) => {
                upnp_gateway = Some(gateway);
                state.gateway = Some(gateway);
                state.wan = wan;
                match opened {
                    Ok(opened) => state.opened = opened,
                    Err(e) => errors.push(format!("UPnP: {e}")),
                }
            }
            Err(e) => errors.push(format!("UPnP: {e}")),
        }
    }
    // NAT-PMP when UPnP did not open it.
    if probes.natpmp_on && state.opened.is_none() {
        let at = probes.natpmp.or_else(|| match (upnp_gateway, local) {
            (Some(IpAddr::V4(g)), _) => Some(SocketAddr::from((g, NATPMP_PORT))),
            (_, Some(IpAddr::V4(l))) => {
                let o = l.octets();
                Some(SocketAddr::from(([o[0], o[1], o[2], 1], NATPMP_PORT)))
            }
            _ => None,
        });
        if let Some(at) = at {
            match natpmp_address(at, probes.wait).await {
                Ok(wan) => {
                    state.gateway.get_or_insert(at.ip());
                    state.wan = state.wan.or(Some(IpAddr::V4(wan)));
                    if open {
                        // The box's address said even when it refuses the mapping (NAT-PMP off in it).
                        match natpmp_map(at, probes.wait, port, port, LEASE).await {
                            Ok((external, lease)) => {
                                state.opened = Some(Opened {
                                    by: Opener::Natpmp,
                                    external_port: external,
                                    lease,
                                    at: Some(at),
                                })
                            }
                            Err(e) => errors.push(format!("NAT-PMP: {e}")),
                        }
                    }
                }
                Err(e) => errors.push(format!("NAT-PMP: {e}")),
            }
        }
    }
    if open && state.opened.is_none() {
        state.open_error = Some(if errors.is_empty() {
            tr!("aucune box n’a répondu", "no box answered")
        } else {
            errors.join(" · ")
        });
    }
    // The address seen from outside.
    for server in &probes.stun {
        if let Some(public) = stun(server, probes.wait).await {
            state.public = Some(public.ip());
            break;
        }
    }
    classify(&mut state);
    state
}

/// The CGNAT, a double NAT, or a machine on the Internet itself, from what was seen.
pub fn classify(state: &mut NetState) {
    state.direct = matches!((state.public, state.local), (Some(p), Some(l)) if p == l);
    let wan_shared = state.wan.is_some_and(not_public);
    let differs = matches!((state.wan, state.public), (Some(w), Some(p)) if w != p);
    state.cgnat = !state.direct && (wan_shared || differs);
}

/// The mapping removed from the box (the access turned off): best effort.
pub async fn close(opened: &Opened, port: u16, probes: &Probes) {
    match opened.by {
        Opener::Natpmp => {
            if let Some(at) = opened.at {
                let _ = natpmp_map(at, probes.wait, port, 0, 0).await;
            }
        }
        Opener::Upnp => {
            if let Some(ssdp) = probes.upnp {
                let _ = upnp_remove(ssdp, probes.wait, opened.external_port).await;
            }
        }
    }
}

// ---------- UPnP ----------

async fn upnp(
    ssdp: SocketAddr,
    wait: Duration,
    local: Ipv4Addr,
    port: u16,
    open: bool,
) -> Result<(IpAddr, Option<IpAddr>, Result<Option<Opened>, String>), String> {
    use igd_next::{PortMappingProtocol, SearchOptions};
    let mut options = SearchOptions::default();
    options.broadcast_address = ssdp;
    if ssdp.ip().is_loopback() {
        options.bind_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    }
    options.timeout = Some(wait);
    options.single_search_timeout = Some(wait);
    let gateway = igd_next::aio::tokio::search_gateway(options)
        .await
        .map_err(|e| e.to_string())?;
    let at = gateway.addr;
    let wan = gateway.get_external_ip().await.ok();
    if !open {
        return Ok((at.ip(), wan, Ok(None)));
    }
    let to = SocketAddr::from((local, port));
    let mut lease = LEASE;
    let mut added = gateway
        .add_port(PortMappingProtocol::UDP, port, to, lease, DESCRIPTION)
        .await;
    // Some boxes take a mapping until removed only.
    if matches!(
        added,
        Err(igd_next::AddPortError::OnlyPermanentLeasesSupported)
    ) {
        lease = 0;
        added = gateway
            .add_port(PortMappingProtocol::UDP, port, to, lease, DESCRIPTION)
            .await;
    }
    // The box found, its address said, even when it refuses the mapping.
    let opened = added
        .map(|()| {
            Some(Opened {
                by: Opener::Upnp,
                external_port: port,
                lease,
                at: Some(at),
            })
        })
        .map_err(|e| e.to_string());
    Ok((at.ip(), wan, opened))
}

async fn upnp_remove(ssdp: SocketAddr, wait: Duration, port: u16) -> Result<(), String> {
    // The gateway found again (its description gives the control address).
    let mut options = igd_next::SearchOptions::default();
    options.broadcast_address = ssdp;
    options.timeout = Some(wait);
    options.single_search_timeout = Some(wait);
    if ssdp.ip().is_loopback() {
        options.bind_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    }
    let gateway = igd_next::aio::tokio::search_gateway(options)
        .await
        .map_err(|e| e.to_string())?;
    gateway
        .remove_port(igd_next::PortMappingProtocol::UDP, port)
        .await
        .map_err(|e| e.to_string())
}

// ---------- NAT-PMP (RFC 6886) ----------

async fn natpmp_ask(
    at: SocketAddr,
    wait: Duration,
    request: &[u8],
    opcode: u8,
) -> Result<Vec<u8>, String> {
    let bind: SocketAddr = if at.ip().is_loopback() {
        (Ipv4Addr::LOCALHOST, 0).into()
    } else {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(bind).await.map_err(|e| e.to_string())?;
    socket.connect(at).await.map_err(|e| e.to_string())?;
    // RFC 6886 § 3.1: 250 ms, doubled at each try; bounded by `wait` here.
    let mut delay = Duration::from_millis(250);
    let started = std::time::Instant::now();
    let mut buf = [0u8; 64];
    while started.elapsed() < wait {
        let _ = socket.send(request).await;
        if let Ok(Ok(n)) = tokio::time::timeout(delay, socket.recv(&mut buf)).await {
            if n >= 8 && buf[0] == 0 && buf[1] == 128 + opcode {
                let result = u16::from_be_bytes([buf[2], buf[3]]);
                if result != 0 {
                    return Err(natpmp_error(result));
                }
                return Ok(buf[..n].to_vec());
            }
        }
        delay *= 2;
    }
    Err(tr!("pas de réponse", "no answer"))
}

fn natpmp_error(code: u16) -> String {
    match code {
        1 => tr!("version non prise en charge", "unsupported version"),
        2 => tr!(
            "refusé (NAT-PMP est coupé dans la box)",
            "not authorized (NAT-PMP is off in the box)"
        ),
        3 => tr!(
            "panne du réseau (la box n’a pas d’adresse sur Internet)",
            "network failure (the box has no address on the Internet)"
        ),
        4 => tr!("plus de ressources dans la box", "out of resources"),
        5 => tr!("demande non prise en charge", "unsupported opcode"),
        n => tr!("erreur {n}", "error {n}"),
    }
}

async fn natpmp_address(at: SocketAddr, wait: Duration) -> Result<Ipv4Addr, String> {
    let answer = natpmp_ask(at, wait, &[0, 0], 0).await?;
    if answer.len() < 12 {
        return Err(tr!("réponse trop courte", "short answer"));
    }
    Ok(Ipv4Addr::new(answer[8], answer[9], answer[10], answer[11]))
}

/// The UDP port `internal` mapped (`lifetime` 0: removed): the external port and the lifetime the box gave.
async fn natpmp_map(
    at: SocketAddr,
    wait: Duration,
    internal: u16,
    external: u16,
    lifetime: u32,
) -> Result<(u16, u32), String> {
    let mut request = vec![0u8, 1, 0, 0];
    request.extend_from_slice(&internal.to_be_bytes());
    request.extend_from_slice(&external.to_be_bytes());
    request.extend_from_slice(&lifetime.to_be_bytes());
    let answer = natpmp_ask(at, wait, &request, 1).await?;
    if answer.len() < 16 {
        return Err(tr!("réponse trop courte", "short answer"));
    }
    let mapped = u16::from_be_bytes([answer[10], answer[11]]);
    let given = u32::from_be_bytes([answer[12], answer[13], answer[14], answer[15]]);
    Ok((mapped, given))
}

// ---------- STUN (RFC 5389, a binding request) ----------

const MAGIC: u32 = 0x2112_A442;

/// The address `server` (`host:port`) sees this machine's UDP from.
pub async fn stun(server: &str, wait: Duration) -> Option<SocketAddr> {
    let to = tokio::time::timeout(wait, tokio::net::lookup_host(server))
        .await
        .ok()?
        .ok()?
        .find(|a| a.is_ipv4())?;
    let bind: SocketAddr = if to.ip().is_loopback() {
        (Ipv4Addr::LOCALHOST, 0).into()
    } else {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(bind).await.ok()?;
    let mut id = [0u8; 12];
    getrandom::fill(&mut id).ok()?;
    let mut request = Vec::with_capacity(20);
    request.extend_from_slice(&1u16.to_be_bytes());
    request.extend_from_slice(&0u16.to_be_bytes());
    request.extend_from_slice(&MAGIC.to_be_bytes());
    request.extend_from_slice(&id);
    let mut buf = [0u8; 512];
    for _ in 0..2 {
        socket.send_to(&request, to).await.ok()?;
        if let Ok(Ok((n, from))) = tokio::time::timeout(wait / 2, socket.recv_from(&mut buf)).await
        {
            if from == to {
                if let Some(seen) = stun_answer(&buf[..n], &id) {
                    return Some(seen);
                }
            }
        }
    }
    None
}

/// The address a STUN answer to the request `id` gives (XOR-MAPPED-ADDRESS, else MAPPED-ADDRESS).
pub fn stun_answer(answer: &[u8], id: &[u8; 12]) -> Option<SocketAddr> {
    if answer.len() < 20 || u16::from_be_bytes([answer[0], answer[1]]) != 0x0101 {
        return None;
    }
    if u32::from_be_bytes([answer[4], answer[5], answer[6], answer[7]]) != MAGIC
        || &answer[8..20] != id
    {
        return None;
    }
    let len = u16::from_be_bytes([answer[2], answer[3]]) as usize;
    let attrs = answer.get(20..20 + len)?;
    let mut at = 0;
    let mut mapped = None;
    while at + 4 <= attrs.len() {
        let kind = u16::from_be_bytes([attrs[at], attrs[at + 1]]);
        let size = u16::from_be_bytes([attrs[at + 2], attrs[at + 3]]) as usize;
        let value = attrs.get(at + 4..at + 4 + size)?;
        if value.len() >= 8 && value[1] == 1 {
            let port = u16::from_be_bytes([value[2], value[3]]);
            let ip = Ipv4Addr::new(value[4], value[5], value[6], value[7]);
            match kind {
                0x0020 => {
                    let m = MAGIC.to_be_bytes();
                    let port = port ^ u16::from_be_bytes([m[0], m[1]]);
                    let ip = Ipv4Addr::from(u32::from(ip) ^ MAGIC);
                    return Some(SocketAddr::V4(SocketAddrV4::new(ip, port)));
                }
                0x0001 => mapped = Some(SocketAddr::V4(SocketAddrV4::new(ip, port))),
                _ => {}
            }
        }
        at += 4 + size.div_ceil(4) * 4;
    }
    mapped
}
