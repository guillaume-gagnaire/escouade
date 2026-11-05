//! `net/fetch` and `net/stream`.

use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::serve::{Cx, Refusal};
use serde_json::{json, Value};

/// reqwest on your machine, `net/fetch` in the module: same source.
pub async fn issue(cx: &Cx, id: &str) -> Result<Value, Refusal> {
    let refused = |e: reqwest::Error| Refusal {
        code: -32004,
        message: e.to_string(),
    };
    let token = cx.secret("token").await?.unwrap_or_default();
    let response = reqwest::Client::new()
        .get(format!("https://api.example.dev/issues/{id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(refused)?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(refused)?;
    Ok(json!({ "status": status, "body": body }))
}

/// Opens a stream: its pieces come as `net/chunk`, its end as `net/end`.
pub async fn follow(cx: &Cx, url: &str) -> Result<Value, Refusal> {
    cx.request("net/stream", json!({ "url": url, "sse": true }))
        .await
}

/// A piece, read: its bytes go back to Escouade, which reads no further than 256 KiB ahead of the plugin.
pub fn chunk(cx: &Cx, piece: &Value) {
    let text = piece["data"].as_str().map(str::len);
    let bytes = piece["dataBase64"].as_str().map(|b64| b64.len() / 4 * 3);
    let bytes = text.or(bytes).unwrap_or_default();
    cx.log("debug", &format!("{bytes} bytes"));
    cx.notify(
        "net/credit",
        json!({ "stream": piece["stream"], "bytes": bytes }),
    );
}

pub fn end(cx: &Cx, end: &Value) {
    if let Some(error) = end["error"].as_str() {
        cx.log("warn", error);
    }
}
