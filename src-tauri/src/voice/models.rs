//! Whisper's models (`escouade-voice-local`'s `models`), and the client of their download, which the settings make.

pub use escouade_voice_local::models::*;

use anyhow::Result;
use std::time::Duration;

/// The client of the download: the proxy and the TLS setting of the app's own requests, no limit
/// on the whole (1.6 GB behind a slow proxy takes its time), but one on a connection that stalls.
pub fn client(settings: &crate::model::Settings) -> Result<reqwest::Client> {
    Ok(crate::tls::app_builder(settings)?
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .build()?)
}
