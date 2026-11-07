//! What Claude Code keeps of an account's sign-in, read-only (ESC-162): the file and the keychain entry it writes. The core
//! holds the accounts, and starts an agent's process again once its sign-in is good (`src-tauri/src/signin.rs`,
//! docs/PROTOCOL.md, « Connexion qui ne se rafraîchit pas »); the plugin reads none of it. Neither a token nor a sign-in
//! leaves this: what is said of one is a fingerprint.

use anyhow::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

/// Whether the process can go on with the sign-in read now (`now`), having started with `launch`:
/// a token in date, or another sign-in than its own (another process refreshed it, or the user
/// signed in again). Unreadable, gone (Claude Code empties it when its refresh token is refused),
/// or the same out of date, it cannot.
pub fn can_go_on(now: Option<&Stored>, launch: Option<&str>) -> bool {
    match now.map(|s| (&s.credentials, s.stamp.as_deref())) {
        Some((Credentials::Token(_), _)) => true,
        Some((Credentials::Expired, stamp)) => stamp != launch,
        _ => false,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// What Claude Code keeps of an account's sign-in to claude.ai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credentials {
    /// Signed in: its OAuth access token.
    Token(String),
    /// Signed in, the token out of date: Claude Code gets a new one when it runs.
    Expired,
    /// Not signed in (never, or with an API key).
    Missing,
}

/// What Claude Code keeps of an account's sign-in, and which sign-in that is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub credentials: Credentials,
    /// A fingerprint of the sign-in (a hash, never the token): another after a new sign-in, the
    /// same while it is out of date. None when not signed in.
    pub stamp: Option<String>,
}

/// A short fingerprint of the sign-in in `oauth` (`claudeAiOauth`): a hash of its tokens, whose
/// first bytes tell one sign-in from another without ever holding a token.
pub fn stamp_of(oauth: &Value) -> Option<String> {
    let token = oauth["accessToken"].as_str().filter(|t| !t.is_empty())?;
    let refresh = oauth["refreshToken"].as_str().unwrap_or_default();
    let hash = Sha256::digest(format!("{token}\n{refresh}").as_bytes());
    Some(hash.iter().take(8).map(|b| format!("{b:02x}")).collect())
}

/// What `creds` (the JSON of `.credentials.json`, or of the keychain's entry) says at `now` (ms): the rule, without reading.
pub fn stored_of(creds: &Value, now: i64) -> Stored {
    let oauth = &creds["claudeAiOauth"];
    let Some(token) = oauth["accessToken"].as_str().filter(|t| !t.is_empty()) else {
        return Stored::none();
    };
    let expired = oauth["expiresAt"].as_i64().is_some_and(|exp| exp < now);
    Stored {
        credentials: if expired {
            Credentials::Expired
        } else {
            Credentials::Token(token.to_string())
        },
        stamp: stamp_of(oauth),
    }
}

impl Stored {
    /// Not signed in.
    fn none() -> Self {
        Stored {
            credentials: Credentials::Missing,
            stamp: None,
        }
    }
}

/// The sign-in the keychain entry holds (`keychain`), else the one of `<dir>/.credentials.json`.
pub async fn credentials_from(keychain: Option<Value>, dir: &Path) -> Result<Credentials> {
    Ok(stored_from(keychain, dir).await?.credentials)
}

/// `credentials_from`, with which sign-in it is.
pub async fn stored_from(keychain: Option<Value>, dir: &Path) -> Result<Stored> {
    let creds: Value = match keychain {
        Some(creds) => creds,
        None => match tokio::fs::read_to_string(dir.join(".credentials.json")).await {
            Ok(text) => serde_json::from_str(&text)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Stored::none()),
            Err(e) => return Err(e.into()),
        },
    };
    Ok(stored_of(&creds, now_ms()))
}

/// What `security find-generic-password -w` answered.
#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq)]
pub enum KeychainAnswer {
    /// The entry's secret: Claude Code's sign-in.
    Entry(Value),
    /// No such entry.
    None,
    /// The entry not given: the user denied the access, or the keychain could not be asked.
    Refused,
}

/// The answer of `security` from its exit code (`code`, none when a signal ended it) and its
/// output: 0 the entry, 44 none (`errSecItemNotFound`, whose low byte the tool exits with), any
/// other a refusal (`errSecAuthFailed` 51, `errSecUserCanceled` 128…).
#[cfg(any(target_os = "macos", test))]
pub fn keychain_answer(code: Option<i32>, stdout: &[u8]) -> Result<KeychainAnswer> {
    Ok(match code {
        Some(0) => KeychainAnswer::Entry(serde_json::from_slice(stdout)?),
        Some(44) => KeychainAnswer::None,
        _ => KeychainAnswer::Refused,
    })
}

/// How long a keychain entry whose access was refused is left alone: reading it again would ask
/// the user again, every minute while the account looks not signed in.
#[cfg(any(target_os = "macos", test))]
pub const KEYCHAIN_REFUSED_MS: i64 = 10 * 60_000;

/// The keychain entries whose access was refused, and when.
#[cfg(any(target_os = "macos", test))]
#[derive(Default)]
pub struct KeychainGate {
    refused: std::collections::HashMap<String, i64>,
}

#[cfg(any(target_os = "macos", test))]
impl KeychainGate {
    /// The entry `service` may be asked for at `now`: not refused in the last ten minutes.
    pub fn open(&self, service: &str, now: i64) -> bool {
        self.refused
            .get(service)
            .is_none_or(|&at| now - at >= KEYCHAIN_REFUSED_MS)
    }

    /// The entry `service` was refused at `now`.
    pub fn refuse(&mut self, service: &str, now: i64) {
        self.refused.insert(service.to_string(), now);
    }
}

/// The keychain entry named `service`, where Claude Code keeps the sign-in on macOS; None when
/// there is no such entry. An entry refused (`KeychainGate`) is an error for ten minutes, without
/// asking again: the last values stand.
#[cfg(target_os = "macos")]
// Its tests read no keychain: `usage::from_keychain` has another body there, and a `pub` of the core is not spared.
#[cfg_attr(test, allow(dead_code))]
pub async fn from_keychain(service: &str) -> Result<Option<Value>> {
    static GATE: std::sync::LazyLock<std::sync::Mutex<KeychainGate>> =
        std::sync::LazyLock::new(Default::default);
    // The guard let go at once: never held across the wait below.
    let open = GATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .open(service, now_ms());
    if !open {
        anyhow::bail!("keychain entry {service} refused a moment ago");
    }
    let out = tokio::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", service, "-w"])
        .stdin(std::process::Stdio::null())
        .output()
        .await?;
    match keychain_answer(out.status.code(), &out.stdout)? {
        KeychainAnswer::Entry(v) => Ok(Some(v)),
        KeychainAnswer::None => Ok(None),
        KeychainAnswer::Refused => {
            GATE.lock()
                .unwrap_or_else(|e| e.into_inner())
                .refuse(service, now_ms());
            anyhow::bail!(
                "keychain entry {service} refused: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )
        }
    }
}

/// The name of the macOS keychain entry Claude Code keeps its sign-in in, run with
/// `CLAUDE_CONFIG_DIR` set to `config_dir_env` (or the value that takes its place, `sign_in_with`):
/// `Claude Code-credentials` without one, else followed by `-` and the first 8 hexadecimal
/// characters of the SHA-256 of the value in NFC.
pub fn keychain_service(config_dir_env: Option<&str>) -> String {
    const SERVICE: &str = "Claude Code-credentials";
    match config_dir_env.filter(|v| !v.is_empty()) {
        None => SERVICE.to_string(),
        Some(dir) => {
            let nfc: String = dir.nfc().collect();
            let hex: String = Sha256::digest(nfc.as_bytes())
                .iter()
                .take(4)
                .map(|b| format!("{b:02x}"))
                .collect();
            format!("{SERVICE}-{hex}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn what_a_sign_in_says_is_read_without_touching_anything() {
        let creds = json!({ "claudeAiOauth": { "accessToken": "tok", "refreshToken": "r", "expiresAt": 100 } });
        assert_eq!(
            stored_of(&creds, 99).credentials,
            Credentials::Token("tok".into())
        );
        let late = stored_of(&creds, 101);
        assert_eq!(late.credentials, Credentials::Expired);
        // The same sign-in out of date, told by its fingerprint, never its token.
        assert_eq!(late.stamp, stored_of(&creds, 99).stamp);
        assert!(!late.stamp.unwrap().contains("tok"));
        assert_eq!(stored_of(&json!({}), 0).credentials, Credentials::Missing);
    }
}
