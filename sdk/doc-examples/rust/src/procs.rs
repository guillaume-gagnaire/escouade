//! `proc/*`.

use escouade_plugin_kit::serve::{Cx, Refusal};
use serde_json::{json, Value};

/// Runs a line of `permissions.commands` in a project: what it writes comes as `proc/out`, its end as `proc/exit`.
pub async fn status(cx: &Cx, project: &str) -> Result<Value, Refusal> {
    let spawned = cx
        .request(
            "proc/spawn",
            json!({
                "command": "git",
                "args": ["status", "--short"],
                "dir": "project",
                "project": project,
            }),
        )
        .await?;
    Ok(spawned["proc"].clone())
}

pub fn out(cx: &Cx, out: &Value) {
    let data = out["data"].as_str().unwrap_or_default();
    cx.log("info", &format!("{}: {data}", out["stream"]));
}

pub fn exit(cx: &Cx, exit: &Value) {
    cx.log("info", &format!("it ended: {}", exit["code"]));
}

/// What it reads on its input; `close` ends that input.
pub async fn feed(cx: &Cx, proc: &Value) -> Result<Value, Refusal> {
    let input = json!({ "proc": proc, "data": "yes\n", "close": true });
    cx.request("proc/write", input).await
}

pub async fn stop(cx: &Cx, proc: &Value) -> Result<Value, Refusal> {
    cx.request("proc/kill", json!({ "proc": proc })).await
}
