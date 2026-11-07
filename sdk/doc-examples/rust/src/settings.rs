//! `secrets/get`, `settings/get` and the journal.

use escouade_plugin_kit::serve::{Cx, Refusal};
use serde_json::{json, Value};

/// One of `permissions.secrets`; none while the user has not given it.
pub async fn token(cx: &Cx) -> Result<Option<String>, Refusal> {
    cx.secret("token").await
}

/// Its settings: with a project, what its `.escouade.yml` gives over what the user chose.
pub async fn channel(cx: &Cx, project: &Value) -> Result<Value, Refusal> {
    let settings = cx
        .request("settings/get", json!({ "project": project }))
        .await?;
    Ok(settings["channel"].clone())
}

/// A line of its journal: `debug`, `info`, `warn` or `error`.
pub fn journal(cx: &Cx, fact: &Value) {
    cx.log("info", &format!("{} received", fact["name"]));
}
