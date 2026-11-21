//! Profiles, the settings of each project and the jokers (Escouade 2.2.5), by their calls. The kit 0.1.4 names them
//! (`Cx::profile_of`, `Cx::secret_of`, `Cx::spawn_typed`) and checks first that Escouade serves them.

use escouade_plugin_kit::serve::{Cx, Refusal};
use serde_json::{json, Value};

/// The profile the project chose (a `profile` setting of `"scope": "project"`): its id, none while it chose none.
pub async fn profile_of(cx: &Cx, project: &str) -> Result<Option<String>, Refusal> {
    let settings = cx
        .request("settings/get", json!({ "project": project }))
        .await?;
    Ok(settings["profile"].as_str().map(str::to_string))
}

/// A secret field of that profile: a configuration on several lines comes whole.
pub async fn secret_of(cx: &Cx, profile: &str, key: &str) -> Result<Option<String>, Refusal> {
    let v = cx
        .request("secrets/get", json!({ "key": key, "profile": profile }))
        .await?;
    Ok(v.as_str().map(str::to_string))
}

/// The tunnel, then its network, given the pid of the first program (`{pid}`).
pub async fn tunnel(cx: &Cx) -> Result<Value, Refusal> {
    let args = [
        "-Urn",
        "openvpn",
        "--config",
        "tunnel.ovpn",
        "--script-security",
        "1",
    ];
    let vpn = cx
        .request("proc/spawn", json!({ "command": "unshare", "args": args }))
        .await?;
    let pid = vpn["pid"].as_u64().unwrap_or_default().to_string();
    let args = ["--configure", "--mtu=65520", pid.as_str(), "tap0"];
    cx.request(
        "proc/spawn",
        json!({ "command": "slirp4netns", "args": args }),
    )
    .await
}
