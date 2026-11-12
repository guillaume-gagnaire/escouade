//! Whisper's models (`escouade-voice-local`'s `models`), and the client of their download, which the settings make.

pub use escouade_voice_local::models::*;

use anyhow::Result;
use std::time::Duration;

/// The client of the download: the proxy and the TLS setting of the app's own requests, no limit
/// on the whole (1.6 GB behind a slow proxy takes its time), but one on a connection that stalls.
pub fn client(settings: &crate::model::Settings) -> Result<reqwest::Client> {
    let mut b = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .danger_accept_invalid_certs(settings.insecure_tls);
    let url = settings.proxy_url.trim();
    if !url.is_empty() {
        let proxy =
            reqwest::Proxy::all(url)?.no_proxy(reqwest::NoProxy::from_string(&settings.no_proxy));
        b = b.proxy(proxy);
    }
    Ok(b.build()?)
}
