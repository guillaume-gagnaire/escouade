//! Escouade's phone side (MO-3, `docs/MOBILE.md`): the Rust of the mobile app.
//!
//! - [`tunnel`]: the app's WireGuard peer (`boringtun`, in user space, no system VPN), a TCP/IP stack (`smoltcp`)
//!   and a local proxy on `127.0.0.1` through which the WebView's WebSocket reaches the core's remote API.
//! - [`notification`]: the opening of the sealed notifications (XChaCha20-Poly1305 under the pairing key), their
//!   format, and the guard against replays.
//! - [`keys`]: the 32-byte keys of the pairing.
//!
//! The Capacitor plugin (`apps/mobile-plugin`) calls it through UniFFI's Kotlin and Swift bindings (the `uniffi`
//! feature, [`ffi`]). Without the default `tunnel` feature, only the notifications are left: what the core needs to
//! seal them (MO-6).

pub mod keys;
pub mod notification;
#[cfg(feature = "tunnel")]
pub mod tunnel;

#[cfg(feature = "uniffi")]
pub mod ffi;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

/// This crate's version, as the app shows it in its « About ».
pub fn version() -> String {
    format!("escouade-mobile {}", env!("CARGO_PKG_VERSION"))
}
