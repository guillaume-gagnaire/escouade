//! The TLS of the app's requests (the proxy, the authorities of the settings), in `escouade-plugins`, whose host reaches
//! the plugins' services with it (docs/CRATES.md); its tests stay here, on the app's fake server.

pub use escouade_plugins::tls::*;

#[cfg(test)]
#[path = "tls_tests.rs"]
mod tests;
