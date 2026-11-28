//! The protocol between `escouade-server` and `escouade-connector` (RP-2, `docs/PRIVATE-NETWORK.md` § 4.2; the spec
//! page « Escouade à distance › Atteindre un réseau privé »).
//!
//! Layer by layer, from the outside in:
//!
//! 1. **The transport** ([`transport`]): a WebSocket over TLS, `wss://<server>:<port C>/connector`, always opened by
//!    the connector, through the company's HTTP proxy when it has one (`CONNECT`, Basic authentication only), with a
//!    file of authorities more (a company's inspection of TLS). The outer certificate plays no part in who is who: a
//!    proxy that opens the TLS sees Noise alone.
//! 2. **The authentication** ([`noise`]): `Noise_IK_25519_ChaChaPoly_BLAKE2s` (the crate `snow`) in the first two
//!    messages of the WebSocket; the connector knows the server's static key, the server learns the connector's from
//!    the first message and accepts it only if it is pinned. Without a key it knows, the server closes after that first
//!    message and says nothing else ([`channel::AcceptError::is_failed_attempt`] for RP-4 to count the tries).
//!    The versions travel inside those two messages ([`version`]): the server accepts its own minor and the two before.
//! 3. **The frames** ([`frame`]): each WebSocket message after the handshake is one Noise message holding one frame:
//!    `open`, `opened`, `refused`, `data`, `close`, `window`, `ping`, `pong`, `ceiling`.
//! 4. **The streams** ([`channel`]): the server opens, the connector accepts or refuses; each stream has a window in
//!    each direction (as HTTP/2: a slow `git clone` does not hold an `npm install`), a `ping` every 15 s, the channel
//!    said cut after 45 s of silence, the bounds of [`limits::Limits`]. [`life`] holds the connector's schedule of
//!    reconnection.
//!
//! Nothing here depends on the core of Escouade nor on Tauri: the connector is a program apart (§ 4.6).

pub mod channel;
pub mod frame;
pub mod life;
pub mod limits;
pub mod noise;
pub mod pairing;
pub mod transport;
pub mod version;

#[cfg(feature = "fuzzing")]
pub mod fuzzing;
#[cfg(all(test, not(feature = "fuzzing")))]
#[path = "fuzzing.rs"]
mod fuzzing;

pub use channel::{
    accept_connector, connect_server, AcceptError, Closed, ConnectError, ConnectorChannel,
    Incoming, OpenError, ServerChannel, Stream, StreamReader, StreamWriter,
};
pub use frame::{Ceiling, Frame, Open, Refusal, RefusalCode, StreamId};
pub use limits::Limits;
pub use noise::{Keypair, PublicKey};
pub use version::{Verdict, Version};

/// The path of the connectors' entry on the server.
pub const PATH: &str = "/connector";
