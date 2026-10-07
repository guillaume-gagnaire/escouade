//! Plan rate limits (5-hour session and weekly windows).

use crate::model::{RateWindow, Settings};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::time::Duration;

pub type Windows = (Option<RateWindow>, Option<RateWindow>);

/// Parses `rate_limits` from a `get_usage` control response, or the body of the OAuth usage
/// endpoint (same shape: `{five_hour: {utilization 0-100, resets_at ISO}, seven_day: …}`).
pub fn parse_windows(v: &Value) -> Windows {
    let parse = |k: &str| {
        let w = v.get(k).filter(|w| w.is_object())?;
        Some(RateWindow {
            pct: w["utilization"].as_f64().unwrap_or(0.0),
            resets_at: w["resets_at"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.timestamp_millis()),
        })
    };
    (parse("five_hour"), parse("seven_day"))
}

/// The app's own requests (quotas, ticket systems): through the proxy, trusting the certificates
/// the system trusts as well as the usual ones, or none checked when TLS verification is off.
pub fn http_client(settings: &Settings) -> Result<reqwest::Client> {
    let mut b = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .danger_accept_invalid_certs(settings.insecure_tls);
    let url = settings.proxy_url.trim();
    if !url.is_empty() {
        let proxy =
            reqwest::Proxy::all(url)?.no_proxy(reqwest::NoProxy::from_string(&settings.no_proxy));
        b = b.proxy(proxy);
    }
    Ok(b.build()?)
}

/// Fallback when no Claude process is running: the endpoint `/usage` relies on, authenticated
/// with the OAuth token Claude Code stores locally (read-only use).
pub async fn fetch_oauth(settings: &Settings) -> Result<Windows> {
    let creds = read_credentials().await?;
    let oauth = &creds["claudeAiOauth"];
    let token = oauth["accessToken"]
        .as_str()
        .ok_or_else(|| anyhow!("pas de connexion claude.ai"))?;
    if oauth["expiresAt"]
        .as_i64()
        .is_some_and(|exp| exp < crate::model::now_ms())
    {
        return Err(anyhow!("jeton OAuth expiré"));
    }
    let body: Value = http_client(settings)?
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", "escouade")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(parse_windows(&body))
}

/// Claude Code's credentials: its file, or on macOS the login Keychain where it keeps them.
async fn read_credentials() -> Result<Value> {
    let path = dirs::home_dir()
        .context("home")?
        .join(".claude")
        .join(".credentials.json");
    match tokio::fs::read_to_string(&path).await {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        #[cfg(target_os = "macos")]
        Err(_) => {
            let out = tokio::process::Command::new("/usr/bin/security")
                .args([
                    "find-generic-password",
                    "-s",
                    "Claude Code-credentials",
                    "-w",
                ])
                .stdin(std::process::Stdio::null())
                .output()
                .await?;
            if !out.status.success() {
                return Err(anyhow!("pas de connexion claude.ai"));
            }
            Ok(serde_json::from_slice(&out.stdout)?)
        }
        #[cfg(not(target_os = "macos"))]
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_usage_windows() {
        let v = json!({
            "five_hour": {"utilization": 62.0, "resets_at": "2026-09-28T01:20:00.467871+00:00"},
            "seven_day": {"utilization": 30, "resets_at": null},
            "seven_day_opus": null
        });
        let (five, week) = parse_windows(&v);
        let five = five.unwrap();
        assert_eq!(five.pct, 62.0);
        assert!(five.resets_at.unwrap() > 1_790_000_000_000);
        assert_eq!(week.unwrap().resets_at, None);
    }
}

/// Minimum delay between two calls to the OAuth usage endpoint (fallback path only).
pub const OAUTH_POLL_MS: i64 = 5 * 60_000;

/// Whether the fallback endpoint may be called again.
pub fn oauth_due(last_call: Option<i64>, now: i64) -> bool {
    last_call.is_none_or(|t| now - t >= OAUTH_POLL_MS)
}

#[cfg(test)]
mod backoff_tests {
    use super::*;

    #[test]
    fn the_fallback_endpoint_is_polled_at_most_every_five_minutes() {
        assert!(oauth_due(None, 0));
        assert!(!oauth_due(Some(1_000), 1_000 + 60_000));
        assert!(oauth_due(Some(1_000), 1_000 + 5 * 60_000));
    }
}
