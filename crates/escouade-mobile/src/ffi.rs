//! What the Capacitor plugin sees, through UniFFI's Kotlin and Swift bindings (`apps/mobile-plugin`): plain records
//! and one object, the keys and the sealed texts as strings. The plugin's TypeScript API
//! (`apps/mobile-plugin/src/definitions.ts`) mirrors it.
//!
//! An error reaches Kotlin as `MobileException.<Variant>` and Swift as `MobileError.<Variant>`: the plugin rejects
//! with the variant's name as the code (`Forged`, `Replayed`…) and its text as the message.

use std::sync::Arc;

use crate::keys::decode_key;
use crate::notification::{self, OpenError, ReplayGuard};
use crate::tunnel::direct::Direct;
use crate::tunnel::pairing;
use crate::tunnel::{Tunnel, TunnelConfig, TunnelError, TunnelState, TunnelStatus};

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum MobileError {
    #[error("{0}")]
    BadKey(String),
    #[error("{0}")]
    BadConfig(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    TooLarge(String),
    #[error("{0}")]
    Malformed(String),
    #[error("{0}")]
    UnknownVersion(String),
    #[error("{0}")]
    Forged(String),
    #[error("{0}")]
    Replayed(String),
    #[error("{0}")]
    TooOld(String),
    #[error("{0}")]
    BadGuardState(String),
}

impl From<TunnelError> for MobileError {
    fn from(e: TunnelError) -> Self {
        let text = e.to_string();
        match e {
            TunnelError::BadKey(_) => MobileError::BadKey(text),
            TunnelError::BadAddress(_) | TunnelError::BadApi(_) | TunnelError::BadEndpoint(_) => {
                MobileError::BadConfig(text)
            }
            TunnelError::Io(_) => MobileError::Io(text),
        }
    }
}

impl From<OpenError> for MobileError {
    fn from(e: OpenError) -> Self {
        let text = e.to_string();
        match e {
            OpenError::BadKey => MobileError::BadKey(text),
            OpenError::TooLarge => MobileError::TooLarge(text),
            OpenError::Malformed => MobileError::Malformed(text),
            OpenError::UnknownVersion(_) => MobileError::UnknownVersion(text),
            OpenError::Forged => MobileError::Forged(text),
            OpenError::Replayed(_) => MobileError::Replayed(text),
            OpenError::TooOld(_) => MobileError::TooOld(text),
        }
    }
}

/// [`TunnelConfig`], for the bindings.
#[derive(Clone, uniffi::Record)]
pub struct MobileTunnelConfig {
    pub private_key: String,
    pub peer_public_key: String,
    pub preshared_key: Option<String>,
    pub endpoint: String,
    pub address: String,
    pub api: String,
    pub keepalive_secs: Option<u16>,
    /// The device's token (ESC-570): the proxy writes it in each request, the WebView never holds it.
    pub device_token: Option<String>,
}

impl From<MobileTunnelConfig> for TunnelConfig {
    fn from(c: MobileTunnelConfig) -> Self {
        TunnelConfig {
            private_key: c.private_key,
            peer_public_key: c.peer_public_key,
            preshared_key: c.preshared_key,
            endpoint: c.endpoint,
            address: c.address,
            api: c.api,
            keepalive_secs: c.keepalive_secs,
            device_token: c.device_token,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileTunnelState {
    Connecting,
    Up,
    Stopped,
    Failed,
}

/// [`TunnelStatus`], for the bindings.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileTunnelStatus {
    pub state: MobileTunnelState,
    pub proxy_url: String,
    pub websocket_url: String,
    pub last_handshake_secs: Option<u64>,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub connections: u32,
    pub error: Option<String>,
}

impl From<TunnelStatus> for MobileTunnelStatus {
    fn from(s: TunnelStatus) -> Self {
        MobileTunnelStatus {
            state: match s.state {
                TunnelState::Connecting => MobileTunnelState::Connecting,
                TunnelState::Up => MobileTunnelState::Up,
                TunnelState::Stopped => MobileTunnelState::Stopped,
                TunnelState::Failed => MobileTunnelState::Failed,
            },
            proxy_url: s.proxy_url,
            websocket_url: s.websocket_url,
            last_handshake_secs: s.last_handshake_secs,
            tx_bytes: s.tx_bytes,
            rx_bytes: s.rx_bytes,
            connections: s.connections,
            error: s.error,
        }
    }
}

/// A space's tunnel and its proxy (one per space the phone is paired with).
#[derive(uniffi::Object)]
pub struct MobileTunnel {
    inner: Tunnel,
}

#[uniffi::export]
impl MobileTunnel {
    /// Starts the tunnel: the first handshake goes out, the proxy listens; [`status`](Self::status) says when it is up.
    #[uniffi::constructor]
    pub fn start(config: MobileTunnelConfig) -> Result<Arc<Self>, MobileError> {
        Ok(Arc::new(MobileTunnel {
            inner: Tunnel::start(&config.into())?,
        }))
    }

    pub fn status(&self) -> MobileTunnelStatus {
        self.inner.status().into()
    }

    /// Closes the proxy and the tunnel. Idempotent.
    pub fn stop(&self) {
        self.inner.stop();
    }
}

/// A space reached without the tunnel (ESC-570: its address on Tailscale, or the e2e's): the same local proxy, its
/// secret and the device's token, to a TCP address.
#[derive(uniffi::Object)]
pub struct MobileDirect {
    inner: Direct,
}

#[uniffi::export]
impl MobileDirect {
    /// Starts the proxy to `api` (`host:port`): `status` gives its address.
    #[uniffi::constructor]
    pub fn start(api: String, device_token: Option<String>) -> Result<Arc<Self>, MobileError> {
        Ok(Arc::new(MobileDirect {
            inner: Direct::start(&api, device_token)?,
        }))
    }

    pub fn status(&self) -> MobileTunnelStatus {
        self.inner.status().into()
    }

    pub fn stop(&self) {
        self.inner.stop();
    }
}

/// The phone's half of a pairing (ESC-570): a key pair made here; its public half goes with the code, its private half
/// stays in the native side until [`finish_pairing`].
#[derive(Clone, uniffi::Record)]
pub struct MobilePairingKey {
    pub private_key: String,
    pub public_key: String,
}

/// What a pairing gives the phone, derived from the secret it shares with the core: never sent.
#[derive(Clone, uniffi::Record)]
pub struct MobilePairing {
    pub device_token: String,
    pub pairing_key: String,
}

/// A key pair for a pairing.
#[uniffi::export]
pub fn new_pairing_key() -> MobilePairingKey {
    let (private_key, public_key) = pairing::new_key();
    MobilePairingKey {
        private_key: private_key.to_string(),
        public_key,
    }
}

/// The device's token and the pairing key, from the phone's private half and the core's public half.
#[uniffi::export]
pub fn finish_pairing(
    private_key: String,
    core_public_key: String,
) -> Result<MobilePairing, MobileError> {
    let (device_token, pairing_key) = pairing::finish(&private_key, &core_public_key)
        .map_err(|e| MobileError::BadKey(e.to_string()))?;
    Ok(MobilePairing {
        device_token: device_token.to_string(),
        pairing_key: pairing_key.to_string(),
    })
}

/// A notification opened, and the replay guard's new state, to keep for the next one.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OpenedNotification {
    pub seq: u64,
    pub at: u64,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub target: Option<String>,
    pub guard_state: String,
}

/// Opens a sealed notification with the pairing key, against the replay guard's state for that pairing (empty for
/// the first). Refused if altered, under another key, replayed or too old; the state to keep is in the answer, and
/// only there: on a refusal, the one given stays good.
#[uniffi::export]
pub fn open_notification(
    pairing_key: String,
    sealed: String,
    guard_state: String,
) -> Result<OpenedNotification, MobileError> {
    let key = decode_key(&pairing_key).map_err(|_| MobileError::from(OpenError::BadKey))?;
    let mut guard = ReplayGuard::from_state(&guard_state)
        .map_err(|e| MobileError::BadGuardState(e.to_string()))?;
    let n = notification::open(&key, &sealed, &mut guard)?;
    Ok(OpenedNotification {
        seq: n.seq,
        at: n.at,
        kind: n.kind,
        title: n.title,
        body: n.body,
        target: n.target,
        guard_state: guard.to_state(),
    })
}

/// This crate's version.
#[uniffi::export]
pub fn mobile_version() -> String {
    crate::version()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{encode_key, random_key};
    use crate::notification::{seal, Notification};

    #[test]
    fn opens_and_threads_the_guard_state() {
        let key = random_key();
        let note = |seq| Notification {
            seq,
            at: 1,
            kind: "question".into(),
            title: "api pose une question".into(),
            body: "Quelle base ?".into(),
            target: None,
        };
        let k = encode_key(&key);
        let first =
            open_notification(k.clone(), seal(&key, &note(1)).unwrap(), String::new()).unwrap();
        assert_eq!(
            (first.seq, first.title.as_str()),
            (1, "api pose une question")
        );
        let again = seal(&key, &note(1)).unwrap();
        assert!(matches!(
            open_notification(k.clone(), again.clone(), first.guard_state.clone()),
            Err(MobileError::Replayed(_))
        ));
        // Without the kept state, the guard cannot know: keeping it is the app's part.
        assert!(open_notification(k.clone(), again, String::new()).is_ok());
        let other = encode_key(&random_key());
        assert!(matches!(
            open_notification(
                other,
                seal(&key, &note(2)).unwrap(),
                first.guard_state.clone()
            ),
            Err(MobileError::Forged(_))
        ));
        assert!(matches!(
            open_notification("x".into(), seal(&key, &note(2)).unwrap(), String::new()),
            Err(MobileError::BadKey(_))
        ));
        assert!(matches!(
            open_notification(k, seal(&key, &note(2)).unwrap(), "nope".into()),
            Err(MobileError::BadGuardState(_))
        ));
        assert!(mobile_version().starts_with("escouade-mobile "));
    }

    #[test]
    fn a_bad_configuration_is_a_bad_config() {
        let c = MobileTunnelConfig {
            private_key: encode_key(&random_key()),
            peer_public_key: encode_key(&random_key()),
            preshared_key: None,
            endpoint: "127.0.0.1:9".into(),
            address: "10.77.0.2/24".into(),
            api: "nowhere".into(),
            keepalive_secs: None,
            device_token: None,
        };
        assert!(matches!(
            MobileTunnel::start(c.clone()),
            Err(MobileError::BadConfig(_))
        ));
        assert!(matches!(
            MobileTunnel::start(MobileTunnelConfig {
                private_key: "".into(),
                ..c
            }),
            Err(MobileError::BadKey(_))
        ));
    }
}
