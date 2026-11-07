//! `ws/*`.

use escouade_plugin_kit::serve::{Cx, Refusal};
use serde_json::{json, Value};

/// Opens a socket and says hello. What comes back arrives as `ws/message`, its end as `ws/closed`.
pub async fn open(cx: &Cx) -> Result<Value, Refusal> {
    let token = cx.secret("token").await?.unwrap_or_default();
    let opened = cx
        .request(
            "ws/open",
            json!({
                "url": "wss://api.example.dev/live",
                "headers": { "authorization": format!("Bearer {token}") },
            }),
        )
        .await?;
    let socket = &opened["socket"];
    cx.request("ws/send", json!({ "socket": socket, "text": "hello" }))
        .await?;
    Ok(socket.clone())
}

pub fn message(cx: &Cx, message: &Value) {
    let text = message["text"].as_str().unwrap_or_default();
    cx.log("info", &format!("the socket says {text}"));
}

pub fn closed(cx: &Cx, closed: &Value) {
    cx.log("info", &format!("closed: {}", closed["code"]));
}

pub async fn close(cx: &Cx, socket: &Value) -> Result<Value, Refusal> {
    cx.request("ws/close", json!({ "socket": socket })).await
}
