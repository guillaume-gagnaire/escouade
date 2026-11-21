//! The phone's side of the tunnel (docs/MOBILE.md, M12: WireGuard built in; `docs/REMOTE.md`, « Le téléphone »).
//!
//! The app holds a WireGuard peer in user space (`boringtun`), with no VPN of the system: only its own traffic goes
//! through it. Inside, a TCP/IP stack (`smoltcp`) opens the connections to the core's remote API, at its address in
//! the tunnel. The WebView cannot speak to that stack: it connects to a **local proxy** on `127.0.0.1`, a port drawn
//! by the system, which carries each connection into the tunnel, byte for byte (the WebSocket of the API, its
//! upgrade included). The proxy forwards only a connection that names the session's secret ([`gate`]).
//!
//! One thread per tunnel, woken by the UDP socket, the local connections and the timers (`mio`): no busy loop, for
//! the battery. [`Tunnel::start`] gives the handle; [`Tunnel::status`] says where it stands; [`Tunnel::stop`], or
//! dropping it, closes everything.

mod device;
mod engine;
pub mod gate;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use zeroize::Zeroizing;

use crate::keys::{decode_key, encode_key, random_key};

/// The keepalive when the configuration gives none: under the 30 s after which many NATs forget a UDP mapping.
pub const DEFAULT_KEEPALIVE_SECS: u16 = 25;

/// What the phone is given at pairing (MO-4's QR code) to reach one space.
#[derive(Clone, PartialEq, Eq)]
pub struct TunnelConfig {
    /// The phone's WireGuard private key (base64).
    pub private_key: String,
    /// The core's WireGuard public key (base64).
    pub peer_public_key: String,
    /// WireGuard's optional preshared key (base64).
    pub preshared_key: Option<String>,
    /// Where the core's peer listens, `host:port` (UDP).
    pub endpoint: String,
    /// The phone's address in the tunnel, `10.77.0.2/24` (a bare address is a /32).
    pub address: String,
    /// The remote API in the tunnel, `10.77.0.1:7878`.
    pub api: String,
    /// WireGuard's persistent keepalive, in seconds ([`DEFAULT_KEEPALIVE_SECS`] when `None`, 0 for none).
    pub keepalive_secs: Option<u16>,
}

impl std::fmt::Debug for TunnelConfig {
    /// Without the keys.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TunnelConfig")
            .field("endpoint", &self.endpoint)
            .field("address", &self.address)
            .field("api", &self.api)
            .field("keepalive_secs", &self.keepalive_secs)
            .finish_non_exhaustive()
    }
}

/// Why a tunnel did not start.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TunnelError {
    #[error("the {0} is not a 32-byte key in base64")]
    BadKey(&'static str),
    #[error("the address in the tunnel is not an IPv4 address with an optional prefix: {0}")]
    BadAddress(String),
    #[error("the API's address in the tunnel is not an IPv4 address and a port: {0}")]
    BadApi(String),
    #[error("the endpoint cannot be resolved: {0}")]
    BadEndpoint(String),
    #[error("the system refused a socket: {0}")]
    Io(String),
}

/// Where a tunnel stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelState {
    /// No handshake yet, or the last one has expired: WireGuard tries again every 5 s.
    Connecting,
    /// A handshake under 3 minutes old: the API is reachable.
    Up,
    /// Stopped by [`Tunnel::stop`].
    Stopped,
    /// The engine stopped on an error of the system ([`TunnelStatus::error`]).
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelStatus {
    pub state: TunnelState,
    /// The base address of the local proxy, its secret included: `http://127.0.0.1:<port>/<secret>`.
    pub proxy_url: String,
    /// The WebSocket of the remote API through the proxy: `ws://127.0.0.1:<port>/<secret>/remote`.
    pub websocket_url: String,
    /// Seconds since the last handshake, when there is a session.
    pub last_handshake_secs: Option<u64>,
    /// Bytes of data sent and received in the tunnel.
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    /// Connections to the proxy open now.
    pub connections: u32,
    /// The error that stopped the engine.
    pub error: Option<String>,
}

/// The path of the remote API's WebSocket on the core (`src-tauri/src/remoteapi/proto.rs`, `PATH`).
pub const API_PATH: &str = "/remote";

/// What the engine writes and the handle reads.
#[derive(Debug, Clone, Default)]
struct Shared {
    up: bool,
    last_handshake_secs: Option<u64>,
    tx_bytes: u64,
    rx_bytes: u64,
    connections: u32,
    error: Option<String>,
    done: bool,
}

/// What the engine is started with, read from a [`TunnelConfig`].
struct Plan {
    private: Zeroizing<[u8; 32]>,
    peer: [u8; 32],
    preshared: Option<Zeroizing<[u8; 32]>>,
    endpoint: SocketAddr,
    address: Ipv4Addr,
    prefix: u8,
    api: SocketAddrV4,
    keepalive: Option<u16>,
}

fn plan(config: &TunnelConfig) -> Result<Plan, TunnelError> {
    let private =
        decode_key(&config.private_key).map_err(|_| TunnelError::BadKey("private key"))?;
    let peer = *decode_key(&config.peer_public_key)
        .map_err(|_| TunnelError::BadKey("peer's public key"))?;
    let preshared = match config.preshared_key.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(k) => Some(decode_key(k).map_err(|_| TunnelError::BadKey("preshared key"))?),
    };
    let bad_address = || TunnelError::BadAddress(config.address.clone());
    let (ip, prefix) = match config.address.trim().split_once('/') {
        Some((ip, prefix)) => (ip, prefix.parse::<u8>().map_err(|_| bad_address())?),
        None => (config.address.trim(), 32),
    };
    let address: Ipv4Addr = ip.parse().map_err(|_| bad_address())?;
    if prefix == 0 || prefix > 32 || address.is_unspecified() {
        return Err(bad_address());
    }
    let api: SocketAddrV4 = config
        .api
        .trim()
        .parse()
        .map_err(|_| TunnelError::BadApi(config.api.clone()))?;
    if api.port() == 0 || api.ip().is_unspecified() || *api.ip() == address {
        return Err(TunnelError::BadApi(config.api.clone()));
    }
    let endpoint = config
        .endpoint
        .trim()
        .to_socket_addrs()
        .map_err(|e| TunnelError::BadEndpoint(format!("{}: {e}", config.endpoint)))?
        .next()
        .ok_or_else(|| TunnelError::BadEndpoint(config.endpoint.clone()))?;
    let keepalive = match config.keepalive_secs {
        None => Some(DEFAULT_KEEPALIVE_SECS),
        Some(0) => None,
        Some(s) => Some(s),
    };
    Ok(Plan {
        private,
        peer,
        preshared,
        endpoint,
        address,
        prefix,
        api,
        keepalive,
    })
}

/// A running tunnel and its proxy.
pub struct Tunnel {
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    waker: Arc<mio::Waker>,
    thread: Mutex<Option<JoinHandle<()>>>,
    proxy: SocketAddr,
    secret: String,
}

impl Tunnel {
    /// Opens the UDP socket and the proxy, sends the first handshake and starts the engine's thread.
    pub fn start(config: &TunnelConfig) -> Result<Tunnel, TunnelError> {
        let plan = plan(config)?;
        let secret = encode_key(&random_key());
        let shared = Arc::new(Mutex::new(Shared::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (engine, waker, proxy) =
            engine::Engine::new(plan, secret.clone(), shared.clone(), stop.clone())?;
        let thread = std::thread::Builder::new()
            .name("escouade-tunnel".into())
            .spawn(move || engine.run())
            .map_err(|e| TunnelError::Io(e.to_string()))?;
        Ok(Tunnel {
            shared,
            stop,
            waker,
            thread: Mutex::new(Some(thread)),
            proxy,
            secret,
        })
    }

    /// The local proxy's address (`127.0.0.1:<port>`).
    pub fn proxy_addr(&self) -> SocketAddr {
        self.proxy
    }

    /// The secret the proxy asks for as the first segment of the path.
    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn status(&self) -> TunnelStatus {
        let s = self
            .shared
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let state = if self.stop.load(Ordering::SeqCst) {
            TunnelState::Stopped
        } else if s.error.is_some() || s.done {
            TunnelState::Failed
        } else if s.up {
            TunnelState::Up
        } else {
            TunnelState::Connecting
        };
        TunnelStatus {
            state,
            proxy_url: format!("http://{}/{}", self.proxy, self.secret),
            websocket_url: format!("ws://{}/{}{API_PATH}", self.proxy, self.secret),
            last_handshake_secs: s.last_handshake_secs,
            tx_bytes: s.tx_bytes,
            rx_bytes: s.rx_bytes,
            connections: s.connections,
            error: s.error,
        }
    }

    /// Waits up to `timeout` for the tunnel to be up (for the tests, and for an app that wants to know before it
    /// connects).
    pub fn wait_up(&self, timeout: std::time::Duration) -> bool {
        let until = std::time::Instant::now() + timeout;
        loop {
            match self.status().state {
                TunnelState::Up => return true,
                TunnelState::Stopped | TunnelState::Failed => return false,
                TunnelState::Connecting if std::time::Instant::now() >= until => return false,
                TunnelState::Connecting => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
    }

    /// Closes the proxy, its connections and the UDP socket. Idempotent.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.waker.wake();
        if let Some(t) = self.thread.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = t.join();
        }
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> TunnelConfig {
        TunnelConfig {
            private_key: encode_key(&random_key()),
            peer_public_key: encode_key(&random_key()),
            preshared_key: None,
            endpoint: "127.0.0.1:51820".into(),
            address: "10.77.0.2/24".into(),
            api: "10.77.0.1:7878".into(),
            keepalive_secs: None,
        }
    }

    #[test]
    fn reads_a_good_configuration() {
        let p = plan(&config()).unwrap();
        assert_eq!((p.address, p.prefix), (Ipv4Addr::new(10, 77, 0, 2), 24));
        assert_eq!(p.api, "10.77.0.1:7878".parse().unwrap());
        assert_eq!(p.keepalive, Some(DEFAULT_KEEPALIVE_SECS));
        assert!(p.preshared.is_none());
        let p = plan(&TunnelConfig {
            address: "10.77.0.2".into(),
            keepalive_secs: Some(0),
            ..config()
        })
        .unwrap();
        assert_eq!((p.prefix, p.keepalive), (32, None));
        let p = plan(&TunnelConfig {
            preshared_key: Some(encode_key(&random_key())),
            ..config()
        })
        .unwrap();
        assert!(p.preshared.is_some());
        assert!(plan(&TunnelConfig {
            preshared_key: Some(" ".into()),
            ..config()
        })
        .unwrap()
        .preshared
        .is_none());
    }

    #[test]
    fn refuses_a_bad_configuration() {
        let bad = |c: TunnelConfig| plan(&c).err().unwrap();
        assert_eq!(
            bad(TunnelConfig {
                private_key: "x".into(),
                ..config()
            }),
            TunnelError::BadKey("private key")
        );
        assert_eq!(
            bad(TunnelConfig {
                peer_public_key: "".into(),
                ..config()
            }),
            TunnelError::BadKey("peer's public key")
        );
        assert_eq!(
            bad(TunnelConfig {
                preshared_key: Some("y".into()),
                ..config()
            }),
            TunnelError::BadKey("preshared key")
        );
        for address in [
            "10.77.0.2/33",
            "10.77.0.2/0",
            "fd00::2/64",
            "0.0.0.0/24",
            "10.77.0.2/x",
            "",
        ] {
            assert!(matches!(
                bad(TunnelConfig {
                    address: address.into(),
                    ..config()
                }),
                TunnelError::BadAddress(_)
            ));
        }
        for api in [
            "10.77.0.1",
            "10.77.0.1:0",
            "0.0.0.0:7878",
            "10.77.0.2:7878",
            "[fd00::1]:7878",
        ] {
            assert!(
                matches!(
                    bad(TunnelConfig {
                        api: api.into(),
                        ..config()
                    }),
                    TunnelError::BadApi(_)
                ),
                "{api}"
            );
        }
        assert!(matches!(
            bad(TunnelConfig {
                endpoint: "no port".into(),
                ..config()
            }),
            TunnelError::BadEndpoint(_)
        ));
    }

    #[test]
    fn the_configuration_never_prints_its_keys() {
        let c = TunnelConfig {
            preshared_key: Some(encode_key(&random_key())),
            ..config()
        };
        let shown = format!("{c:?}");
        for key in [
            &c.private_key,
            &c.peer_public_key,
            c.preshared_key.as_ref().unwrap(),
        ] {
            assert!(!shown.contains(key.as_str()));
        }
    }
}
