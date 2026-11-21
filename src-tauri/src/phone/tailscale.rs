//! Tailscale on the machine (ESC-567, M12 of `docs/MOBILE.md`, `docs/REMOTE.md` « Alternative sans WireGuard dans
//! l'app »): when it is there, the phone that has it too reaches the API at the machine's Tailscale address, with no
//! port to open in the box, CGNAT included. Escouade does not redo Tailscale: it finds its address and serves there.
//!
//! Found by its command (`tailscale ip -4`, then `status --json` for the machine's name), where it is installed or on
//! the `PATH`; without the command, by the route to Tailscale's own address (100.100.100.100), which only a machine of
//! a tailnet sends from an address of 100.64.0.0/10.

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
            // Without its command: the route to Tailscale's own address.
            let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
            socket
                .connect(SocketAddr::from(([100, 100, 100, 100], 53)))
                .ok()?;
            match socket.local_addr().ok()?.ip() {
                IpAddr::V4(a) if shared(a) => Some(Found {
                    address: a,
                    name: None,
                }),
                _ => None,
            }
        }
    }
}
