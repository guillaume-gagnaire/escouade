//! `fs/*`, and `files/read` for the plugin's own folder.

use escouade_plugin_kit::serve::{Cx, Refusal};
use serde_json::{json, Value};

/// Writes in its own temporary folder (`permissions.files`, root `temp`).
pub async fn keep(cx: &Cx, text: &str) -> Result<Value, Refusal> {
    let note = json!({ "root": "temp", "path": "notes.txt", "text": text, "append": true });
    cx.request("fs/write", note).await
}

/// Reads it back, what is there and what the host calls it.
pub async fn notes(cx: &Cx) -> Result<Value, Refusal> {
    let file = json!({ "root": "temp", "path": "notes.txt" });
    let read = cx.request("fs/read", file.clone()).await?;
    let stat = cx.request("fs/stat", file.clone()).await?;
    let canon = cx.request("fs/canon", file).await?;
    let list = cx
        .request("fs/list", json!({ "root": "temp", "path": "." }))
        .await?;
    Ok(json!({ "read": read, "stat": stat, "canon": canon, "list": list }))
}

/// A file of the plugin's own folder.
pub async fn words(cx: &Cx) -> Result<Value, Refusal> {
    cx.request("files/read", json!({ "path": "data/words.txt" }))
        .await
}
