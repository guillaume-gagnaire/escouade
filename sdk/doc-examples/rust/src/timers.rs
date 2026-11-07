//! `timer/*`, through the kit's `sleep` and `timeout`: tokio's on your machine, the host's in the module.

use escouade_plugin_kit::serve::{Cx, Refusal};
use escouade_plugin_kit::time::{sleep, timeout};
use serde_json::{json, Value};
use std::time::Duration;

/// Other calls go on while this one waits.
pub async fn later(cx: &Cx) -> Value {
    sleep(Duration::from_secs(1)).await;
    cx.log("info", "one second later");
    json!("rested")
}

/// A timeout on anything that waits.
pub async fn patient(cx: &Cx) -> Result<Option<String>, Refusal> {
    match timeout(Duration::from_secs(2), cx.secret("token")).await {
        Ok(secret) => secret,
        Err(elapsed) => Err(Refusal {
            code: -32004,
            message: elapsed.to_string(),
        }),
    }
}
