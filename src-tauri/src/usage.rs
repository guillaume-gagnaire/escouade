//! Plan rate limits (5-hour session and weekly windows), for each Claude account: told by the
//! account's processes (`get_usage`, `rate_limit_event`), else read from the OAuth usage endpoint
//! with the sign-in Claude Code keeps for the account (its folder, or the macOS keychain).

use crate::accounts;
use crate::board;
use crate::i18n::Lang;
use crate::model::{
    Account, AccountUsage, Hold, RateWindow, SavedPause, SavedWindows, Settings, SignInProblem,
    UsageSnapshot,
};
use anyhow::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use unicode_normalization::UnicodeNormalization;

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

/// The endpoint `/usage` relies on.
pub const USAGE_API: &str = "https://api.anthropic.com/api/oauth/usage";

/// Where the usage endpoint is asked: Anthropic's; in a debug build, `ESCOUADE_USAGE_API` when
/// set (end-to-end tests); in unit tests a closed port, so that none ever reaches Anthropic's
/// with the sign-in of the machine's user.
pub fn api_from_env() -> String {
    if cfg!(test) {
        // The discard port, closed: refused at once.
        return "http://127.0.0.1:9/api/oauth/usage".into();
    }
    if cfg!(debug_assertions) {
        if let Some(api) = std::env::var("ESCOUADE_USAGE_API")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            return api;
        }
    }
    USAGE_API.into()
}

/// Asks the usage endpoint at `api` for the windows of the account signed in with `token`.
pub async fn fetch_oauth(settings: &Settings, api: &str, token: &str) -> Result<Windows> {
    let body: Value = http_client(settings)?
        .get(api)
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

/// `read_credentials`, with which sign-in it is.
pub async fn read_stored(dir: &Path, keychain_service: &str) -> Result<Stored> {
    stored_from(from_keychain(keychain_service).await?, dir).await
}

/// A short fingerprint of the sign-in in `oauth` (`claudeAiOauth`): a hash of its tokens, whose
/// first bytes tell one sign-in from another without ever holding a token.
fn stamp_of(oauth: &Value) -> Option<String> {
    let token = oauth["accessToken"].as_str().filter(|t| !t.is_empty())?;
    let refresh = oauth["refreshToken"].as_str().unwrap_or_default();
    let hash = Sha256::digest(format!("{token}\n{refresh}").as_bytes());
    Some(hash.iter().take(8).map(|b| format!("{b:02x}")).collect())
}

/// The account's sign-in, read-only, where Claude Code reads it: on macOS the keychain entry named
/// `keychain_service`, `<dir>/.credentials.json` only without one (its « keychain with plaintext
/// fallback »: a file left behind does not hide the entry); elsewhere the file.
pub async fn read_credentials(dir: &Path, keychain_service: &str) -> Result<Credentials> {
    credentials_from(from_keychain(keychain_service).await?, dir).await
}

/// The sign-in the keychain entry holds (`keychain`), else the one of `<dir>/.credentials.json`.
async fn credentials_from(keychain: Option<Value>, dir: &Path) -> Result<Credentials> {
    Ok(stored_from(keychain, dir).await?.credentials)
}

/// `credentials_from`, with which sign-in it is.
async fn stored_from(keychain: Option<Value>, dir: &Path) -> Result<Stored> {
    let missing = Stored {
        credentials: Credentials::Missing,
        stamp: None,
    };
    let creds: Value = match keychain {
        Some(creds) => creds,
        None => match tokio::fs::read_to_string(dir.join(".credentials.json")).await {
            Ok(text) => serde_json::from_str(&text)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(missing),
            Err(e) => return Err(e.into()),
        },
    };
    let oauth = &creds["claudeAiOauth"];
    let Some(token) = oauth["accessToken"].as_str().filter(|t| !t.is_empty()) else {
        return Ok(missing);
    };
    let expired = oauth["expiresAt"]
        .as_i64()
        .is_some_and(|exp| exp < crate::model::now_ms());
    Ok(Stored {
        credentials: if expired {
            Credentials::Expired
        } else {
            Credentials::Token(token.to_string())
        },
        stamp: stamp_of(oauth),
    })
}

/// What `security find-generic-password -w` answered.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
enum KeychainAnswer {
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
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn keychain_answer(code: Option<i32>, stdout: &[u8]) -> Result<KeychainAnswer> {
    Ok(match code {
        Some(0) => KeychainAnswer::Entry(serde_json::from_slice(stdout)?),
        Some(44) => KeychainAnswer::None,
        _ => KeychainAnswer::Refused,
    })
}

/// How long a keychain entry whose access was refused is left alone: reading it again would ask
/// the user again, every minute while the account looks not signed in.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const KEYCHAIN_REFUSED_MS: i64 = 10 * 60_000;

/// The keychain entries whose access was refused, and when.
#[derive(Default)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
struct KeychainGate {
    refused: std::collections::HashMap<String, i64>,
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
impl KeychainGate {
    /// The entry `service` may be asked for at `now`: not refused in the last ten minutes.
    fn open(&self, service: &str, now: i64) -> bool {
        self.refused
            .get(service)
            .is_none_or(|&at| now - at >= KEYCHAIN_REFUSED_MS)
    }

    /// The entry `service` was refused at `now`.
    fn refuse(&mut self, service: &str, now: i64) {
        self.refused.insert(service.to_string(), now);
    }
}

/// The keychain entry named `service`, where Claude Code keeps the sign-in on macOS; None when
/// there is no such entry. An entry refused (`KeychainGate`) is an error for ten minutes, without
/// asking again: the last values stand. Never in unit tests: the entry without a hash is the
/// machine user's own (and reading it may ask them for their password).
#[cfg(all(target_os = "macos", not(test)))]
async fn from_keychain(service: &str) -> Result<Option<Value>> {
    static GATE: std::sync::LazyLock<parking_lot::Mutex<KeychainGate>> =
        std::sync::LazyLock::new(Default::default);
    // The guard let go at once: never held across the wait below.
    let open = GATE.lock().open(service, crate::model::now_ms());
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
            GATE.lock().refuse(service, crate::model::now_ms());
            anyhow::bail!(
                "keychain entry {service} refused: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )
        }
    }
}

/// Elsewhere Claude Code keeps it in the file only (and unit tests read no keychain).
#[cfg(any(not(target_os = "macos"), test))]
async fn from_keychain(_service: &str) -> Result<Option<Value>> {
    Ok(None)
}

/// Where Claude Code keeps an account's sign-in: the folder of its `.credentials.json`, and the
/// name of its macOS keychain entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignIn {
    pub dir: PathBuf,
    pub service: String,
}

/// The variable that names Claude Code's configuration folder.
const CONFIG_DIR_VAR: &str = "CLAUDE_CONFIG_DIR";
/// The variable that, when set, takes the place of `CLAUDE_CONFIG_DIR` for the sign-in alone.
const SECURE_STORAGE_VAR: &str = "CLAUDE_SECURESTORAGE_CONFIG_DIR";

/// `sign_in_with` in the app's environment. In unit tests, a home folder that does not exist and
/// none of the variables: Principal's sign-in is never the machine user's own (tests run inside
/// a Claude Code session may have its CLAUDE_CONFIG_DIR).
pub fn sign_in(account: &Account) -> SignIn {
    if cfg!(test) {
        let home = std::env::temp_dir().join("escouade-tests-no-home");
        return sign_in_with(account, None, None, &home);
    }
    let var = |k: &str| std::env::var(k).ok();
    sign_in_with(
        account,
        var(CONFIG_DIR_VAR).as_deref(),
        var(SECURE_STORAGE_VAR).as_deref(),
        &dirs::home_dir().unwrap_or_default(),
    )
}

/// Where the account's Claude Code keeps its sign-in, its processes running with the app's
/// `CLAUDE_CONFIG_DIR` (`config_env`, Principal's) and `CLAUDE_SECURESTORAGE_CONFIG_DIR`
/// (`secure_env`, inherited by every account) besides its own folder, as Claude Code finds it.
pub fn sign_in_with(
    account: &Account,
    config_env: Option<&str>,
    secure_env: Option<&str>,
    home: &Path,
) -> SignIn {
    match secure_env {
        // Set (even empty), it comes first for every process, whatever its CLAUDE_CONFIG_DIR: the
        // accounts then share one sign-in.
        Some(dir) => SignIn {
            dir: if dir.is_empty() {
                home.join(".claude")
            } else {
                PathBuf::from(dir)
            },
            service: keychain_service(Some(dir)),
        },
        // The CLAUDE_CONFIG_DIR its processes run with: its own, else the app's.
        None => SignIn {
            dir: accounts::config_dir_with(account, config_env, home),
            service: keychain_service(accounts::own_dir(account).or(config_env)),
        },
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

/// What a reading of an account's quota found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reading {
    /// Its windows (one not told keeps its last reading).
    Windows(Windows),
    /// No sign-in to ask the endpoint with.
    NotSignedIn,
    /// Its sign-in is out of date.
    Expired,
}

/// The account's quota from the endpoint at `api`, with its sign-in: Err when it could not be
/// read (the network, the server, a file unreadable), the last values standing then.
pub async fn read_oauth(settings: &Settings, api: &str, sign_in: &SignIn) -> Result<Reading> {
    Ok(
        match read_credentials(&sign_in.dir, &sign_in.service).await? {
            Credentials::Missing => Reading::NotSignedIn,
            Credentials::Expired => Reading::Expired,
            Credentials::Token(token) => {
                Reading::Windows(fetch_oauth(settings, api, &token).await?)
            }
        },
    )
}

/// What `AccountUsage::reason` says of a problem, in `lang`.
pub fn reason(problem: SignInProblem, lang: Lang) -> String {
    match problem {
        SignInProblem::NotSignedIn => tr_in!(lang, "Pas connecté", "Not signed in"),
        SignInProblem::Expired => tr_in!(
            lang,
            "Connexion expirée : relance Claude Code pour ce compte.",
            "Sign-in expired: run Claude Code again for this account."
        ),
    }
}

impl UsageSnapshot {
    pub fn account(&self, id: &str) -> Option<&AccountUsage> {
        self.accounts.iter().find(|a| a.id == id)
    }

    /// The account's entry, made when it has none yet.
    pub fn account_mut(&mut self, id: &str) -> &mut AccountUsage {
        let at = match self.accounts.iter().position(|a| a.id == id) {
            Some(at) => at,
            None => {
                self.accounts.push(AccountUsage {
                    id: id.to_string(),
                    ..Default::default()
                });
                self.accounts.len() - 1
            }
        };
        &mut self.accounts[at]
    }

    /// What a reading of the account's quota found, at `now`.
    pub fn record(&mut self, id: &str, reading: Reading, now: i64) {
        let a = self.account_mut(id);
        match reading {
            Reading::Windows((five, week)) => {
                a.five_hour = five.or(a.five_hour);
                a.seven_day = week.or(a.seven_day);
                a.updated_at = now;
                // Read: signed in.
                a.problem = None;
            }
            // Its windows stay as last read: they still say what it used until their end.
            Reading::NotSignedIn => a.problem = Some(SignInProblem::NotSignedIn),
            Reading::Expired => a.problem = Some(SignInProblem::Expired),
        }
    }

    /// The accounts as the settings list them (in their order; one new without any reading, one
    /// removed gone), each told in the interface's language, the current one chosen and its
    /// windows put first (`five_hour`, `seven_day`: what the status bar and the autopilot read).
    pub fn settle(&mut self, settings: &Settings, now: i64) {
        let mut before = std::mem::take(&mut self.accounts);
        self.accounts = settings
            .accounts
            .iter()
            .map(|a| match before.iter().position(|u| u.id == a.id) {
                Some(at) => before.swap_remove(at),
                None => AccountUsage {
                    id: a.id.clone(),
                    ..Default::default()
                },
            })
            .collect();
        let lang = crate::i18n::ui();
        for a in &mut self.accounts {
            // Not read yet: taken as signed in, rather than as an account to avoid.
            a.connected = a.problem != Some(SignInProblem::NotSignedIn);
            a.reason = a.problem.map(|p| reason(p, lang));
        }
        let threshold = board::quota_threshold(settings.quota_pause);
        self.current = accounts::pick(settings, &self.accounts, threshold, None, now);
        if let Some(current) = self.account(&self.current).cloned() {
            self.five_hour = current.five_hour;
            self.seven_day = current.seven_day;
            self.updated_at = current.updated_at;
        }
    }

    /// The windows to keep across a restart, with what else holds the autopilot back: Principal's
    /// where the versions before the accounts kept the only ones, the others' by account.
    pub fn saved(&self, mut hold: Hold) -> SavedPause {
        // Of an account gone from the settings, nothing is kept.
        hold.accounts
            .retain(|id, _| self.accounts.iter().any(|a| a.id == *id));
        let windows = |a: &AccountUsage| SavedWindows {
            five_hour: a.five_hour,
            seven_day: a.seven_day,
        };
        let principal = self.account(accounts::PRINCIPAL).map(windows);
        SavedPause {
            five_hour: principal.and_then(|w| w.five_hour),
            seven_day: principal.and_then(|w| w.seven_day),
            hold,
            accounts: self
                .accounts
                .iter()
                .filter(|a| a.id != accounts::PRINCIPAL)
                .filter(|a| a.five_hour.is_some() || a.seven_day.is_some())
                .map(|a| (a.id.clone(), windows(a)))
                .collect(),
        }
    }

    /// The quotas as the last run left them (`saved`), on the accounts of the settings, keeping
    /// the windows `keep` keeps (those still going on).
    pub fn restored(
        saved: &SavedPause,
        settings: &Settings,
        keep: impl Fn(Option<RateWindow>) -> Option<RateWindow>,
        now: i64,
    ) -> Self {
        let mut usage = Self::default();
        for a in &settings.accounts {
            let w = if a.id == accounts::PRINCIPAL {
                SavedWindows {
                    five_hour: saved.five_hour,
                    seven_day: saved.seven_day,
                }
            } else {
                saved.accounts.get(&a.id).copied().unwrap_or_default()
            };
            let entry = usage.account_mut(&a.id);
            entry.five_hour = keep(w.five_hour);
            entry.seven_day = keep(w.seven_day);
        }
        usage.settle(settings, now);
        usage
    }
}

/// Minimum delay between two calls to the OAuth usage endpoint for one account (fallback path
/// only).
pub const OAUTH_POLL_MS: i64 = 5 * 60_000;

/// Whether the fallback endpoint may be called again.
pub fn oauth_due(last_call: Option<i64>, now: i64) -> bool {
    last_call.is_none_or(|t| now - t >= OAUTH_POLL_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::fake::FakeServer;
    use crate::model::now_ms;
    use crate::paths::test_dir;
    use serde_json::json;
    use std::collections::BTreeMap;

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

    #[test]
    fn the_fallback_endpoint_is_polled_at_most_every_five_minutes() {
        assert!(oauth_due(None, 0));
        assert!(!oauth_due(Some(1_000), 1_000 + 60_000));
        assert!(oauth_due(Some(1_000), 1_000 + 5 * 60_000));
    }

    #[test]
    fn unit_tests_never_ask_anthropics_endpoint_nor_read_the_machine_users_sign_in() {
        let api = api_from_env();
        assert!(api.starts_with("http://127.0.0.1:"), "{api}");
        assert_ne!(api, USAGE_API);
        let principal = sign_in(&accounts::principal());
        assert_eq!(
            principal.dir,
            std::env::temp_dir()
                .join("escouade-tests-no-home")
                .join(".claude")
        );
        assert!(!principal.dir.exists());
    }

    // The values Claude Code's own hash gives (node:crypto, sha256 of the NFC value).
    #[test]
    fn the_keychain_entry_is_named_after_the_accounts_folder_as_claude_code_names_it() {
        assert_eq!(keychain_service(None), "Claude Code-credentials");
        // Set empty: as if it were not.
        assert_eq!(keychain_service(Some("")), "Claude Code-credentials");
        assert_eq!(
            keychain_service(Some(r"C:\Users\x\.escouade\claude\pro")),
            "Claude Code-credentials-a2efd1c3"
        );
        assert_eq!(
            keychain_service(Some("/Users/x/.escouade/claude/pro")),
            "Claude Code-credentials-f3a2dcff"
        );
        // An « é » written as « e » and its accent (as macOS may give a folder's name) is hashed
        // as the single character.
        let composed = "/Users/x/.escouade/claude/\u{e9}quipe";
        let decomposed = "/Users/x/.escouade/claude/e\u{301}quipe";
        assert_ne!(composed, decomposed);
        assert_eq!(
            keychain_service(Some(decomposed)),
            "Claude Code-credentials-96e3c424"
        );
        assert_eq!(
            keychain_service(Some(decomposed)),
            keychain_service(Some(composed))
        );
    }

    fn pro() -> Account {
        Account {
            id: "pro".into(),
            name: "Pro".into(),
            config_dir: "/Users/x/.escouade/claude/pro".into(),
            ..Default::default()
        }
    }

    #[test]
    fn each_account_keeps_its_sign_in_in_its_folder_principal_in_the_apps() {
        let home = Path::new("/Users/x");
        let principal = accounts::principal();
        assert_eq!(
            sign_in_with(&principal, None, None, home),
            SignIn {
                dir: home.join(".claude"),
                service: "Claude Code-credentials".into(),
            }
        );
        // The app run with a CLAUDE_CONFIG_DIR: Principal's processes inherit it.
        assert_eq!(
            sign_in_with(&principal, Some("/tmp/claude-e2e"), None, home),
            SignIn {
                dir: PathBuf::from("/tmp/claude-e2e"),
                service: "Claude Code-credentials-653ed09a".into(),
            }
        );
        // Another account's is its own folder, whatever the app's.
        let own = SignIn {
            dir: PathBuf::from("/Users/x/.escouade/claude/pro"),
            service: "Claude Code-credentials-f3a2dcff".into(),
        };
        assert_eq!(
            sign_in_with(&pro(), Some("/tmp/claude-e2e"), None, home),
            own
        );
        assert_eq!(sign_in_with(&pro(), None, None, home), own);
    }

    #[test]
    fn an_inherited_secure_storage_folder_comes_first_as_claude_code_has_it() {
        let home = Path::new("/Users/x");
        // Set, it is where every account's processes keep their sign-in.
        let shared = SignIn {
            dir: PathBuf::from("/Users/x/.escouade/claude/pro"),
            service: "Claude Code-credentials-f3a2dcff".into(),
        };
        let secure = Some("/Users/x/.escouade/claude/pro");
        assert_eq!(
            sign_in_with(&accounts::principal(), None, secure, home),
            shared
        );
        assert_eq!(
            sign_in_with(&pro(), Some("/tmp/claude-e2e"), secure, home),
            shared
        );
        let other = Account {
            config_dir: "/Users/x/.escouade/claude/perso".into(),
            ..pro()
        };
        assert_eq!(sign_in_with(&other, None, secure, home), shared);
        // Set empty: the home folder's, and the entry without a hash.
        assert_eq!(
            sign_in_with(&pro(), None, Some(""), home),
            SignIn {
                dir: home.join(".claude"),
                service: "Claude Code-credentials".into(),
            }
        );
    }

    fn credentials(dir: &Path, v: Value) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(".credentials.json"), v.to_string()).unwrap();
    }

    // A keychain entry nobody has: on macOS, the lookup there finds nothing either.
    const NO_ENTRY: &str = "Claude Code-credentials-k2-test-none";

    #[tokio::test]
    async fn the_sign_in_is_read_from_the_accounts_folder_present_absent_or_expired() {
        let dir = test_dir("usage-credentials");
        let (signed, absent, expired, other) = (
            dir.join("signed"),
            dir.join("absent"),
            dir.join("expired"),
            dir.join("other"),
        );
        let later = now_ms() + 3_600_000;
        credentials(
            &signed,
            json!({ "claudeAiOauth": { "accessToken": "tok-pro", "expiresAt": later } }),
        );
        std::fs::create_dir_all(&absent).unwrap();
        credentials(
            &expired,
            json!({ "claudeAiOauth": { "accessToken": "tok-old", "expiresAt": now_ms() - 1_000 } }),
        );
        // Only the sign-ins of MCP servers: not signed in to claude.ai.
        credentials(
            &other,
            json!({ "mcpOAuth": { "x": { "accessToken": "t" } } }),
        );
        let read = |d: PathBuf| async move { read_credentials(&d, NO_ENTRY).await.unwrap() };
        assert_eq!(read(signed).await, Credentials::Token("tok-pro".into()));
        assert_eq!(read(absent).await, Credentials::Missing);
        assert_eq!(read(expired).await, Credentials::Expired);
        assert_eq!(read(other).await, Credentials::Missing);
        // A file that cannot be read: an error, not a sign-out.
        let broken = dir.join("broken");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join(".credentials.json"), "{ pas du json").unwrap();
        assert!(read_credentials(&broken, NO_ENTRY).await.is_err());
    }

    #[tokio::test]
    async fn one_sign_in_is_told_from_another_by_a_stamp_that_is_never_the_token() {
        let dir = test_dir("usage-stamp");
        let later = now_ms() + 3_600_000;
        let sign_in = |token: &str, expires_at: i64| json!({ "claudeAiOauth": { "accessToken": token, "refreshToken": "ref", "expiresAt": expires_at } });
        let stamp = |d: PathBuf| async move { read_stored(&d, NO_ENTRY).await.unwrap().stamp };
        credentials(&dir, sign_in("tok-old", later));
        let first = stamp(dir.clone()).await.expect("signed in: a stamp");
        assert_eq!(first.len(), 16);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!first.contains("tok"));
        // Read again, the same sign-in: the same stamp.
        assert_eq!(stamp(dir.clone()).await.as_deref(), Some(first.as_str()));
        // Out of date, it is still that sign-in: signed in, and told by the same stamp, which
        // is how a window tells it from the one the user is about to make.
        credentials(&dir, sign_in("tok-old", now_ms() - 1_000));
        let stored = read_stored(&dir, NO_ENTRY).await.unwrap();
        assert_eq!(stored.credentials, Credentials::Expired);
        assert_eq!(stored.stamp.as_deref(), Some(first.as_str()));
        // A new sign-in, another token: another stamp.
        credentials(&dir, sign_in("tok-new", later));
        let second = stamp(dir.clone()).await.unwrap();
        assert_ne!(second, first);
        // Not signed in (nothing there, or only the sign-ins of MCP servers): none.
        let nothing = dir.join("nothing");
        std::fs::create_dir_all(&nothing).unwrap();
        assert_eq!(stamp(nothing).await, None);
        let other = dir.join("other");
        credentials(
            &other,
            json!({ "mcpOAuth": { "x": { "accessToken": "t" } } }),
        );
        assert_eq!(stamp(other).await, None);
        // The keychain's entry, when there is one, is the sign-in the stamp is of.
        let entry = sign_in("tok-keychain", later);
        let kept = stored_from(Some(entry), &dir).await.unwrap();
        assert_eq!(kept.credentials, Credentials::Token("tok-keychain".into()));
        assert_ne!(kept.stamp.unwrap(), second);
    }

    #[tokio::test]
    async fn on_macos_the_keychain_entry_comes_before_a_file_left_in_the_folder() {
        let dir = test_dir("usage-keychain-first");
        // A file left behind, out of date.
        credentials(
            &dir,
            json!({ "claudeAiOauth": { "accessToken": "tok-file", "expiresAt": now_ms() - 1_000 } }),
        );
        let entry = json!({ "claudeAiOauth": { "accessToken": "tok-keychain", "expiresAt": now_ms() + 60_000 } });
        assert_eq!(
            credentials_from(Some(entry), &dir).await.unwrap(),
            Credentials::Token("tok-keychain".into())
        );
        // No entry: the file.
        assert_eq!(
            credentials_from(None, &dir).await.unwrap(),
            Credentials::Expired
        );
    }

    #[test]
    fn the_keychain_tells_an_entry_none_or_a_refusal() {
        let entry = br#"{"claudeAiOauth":{"accessToken":"tok"}}"#;
        assert_eq!(
            keychain_answer(Some(0), entry).unwrap(),
            KeychainAnswer::Entry(json!({ "claudeAiOauth": { "accessToken": "tok" } }))
        );
        assert_eq!(
            keychain_answer(Some(44), b"").unwrap(),
            KeychainAnswer::None
        );
        // Denied by the user, not allowed to ask, ended by a signal.
        for code in [Some(51), Some(128), Some(36), None] {
            assert_eq!(
                keychain_answer(code, b"").unwrap(),
                KeychainAnswer::Refused,
                "{code:?}"
            );
        }
        // An entry that is not Claude Code's sign-in: an error, the last values standing.
        assert!(keychain_answer(Some(0), b"pas du json").is_err());
    }

    #[test]
    fn an_entry_refused_is_not_asked_for_again_for_ten_minutes() {
        let mut gate = KeychainGate::default();
        let (pro, perso) = (
            "Claude Code-credentials-a2efd1c3",
            "Claude Code-credentials",
        );
        assert!(gate.open(pro, 1_000));
        gate.refuse(pro, 1_000);
        assert!(!gate.open(pro, 1_001));
        assert!(!gate.open(pro, 1_000 + 10 * 60_000 - 1));
        assert!(gate.open(pro, 1_000 + 10 * 60_000));
        // Another entry is asked for as usual.
        assert!(gate.open(perso, 1_001));
    }

    #[tokio::test]
    async fn the_endpoint_is_asked_with_the_accounts_token() {
        let server = FakeServer::start().await;
        server.on(
            "GET",
            "/api/oauth/usage",
            200,
            json!({ "five_hour": { "utilization": 40.0, "resets_at": "2026-09-28T01:20:00+00:00" },
                    "seven_day": { "utilization": 7.0, "resets_at": null } }),
        );
        let api = format!("{}/api/oauth/usage", server.url);
        let settings = Settings::default();
        let (five, week) = fetch_oauth(&settings, &api, "tok-pro").await.unwrap();
        assert_eq!(five.map(|w| w.pct), Some(40.0));
        assert_eq!(week.map(|w| w.pct), Some(7.0));
        let req = &server.requests()[0];
        assert_eq!(
            req.headers.get("authorization").map(String::as_str),
            Some("Bearer tok-pro")
        );
        assert_eq!(
            req.headers.get("anthropic-beta").map(String::as_str),
            Some("oauth-2025-04-20")
        );
        // Through `read_oauth`: the sign-in read from the folder first.
        let dir = test_dir("usage-read-oauth");
        let sign_in = |d: &Path| SignIn {
            dir: d.to_path_buf(),
            service: NO_ENTRY.into(),
        };
        credentials(
            &dir.join("pro"),
            json!({ "claudeAiOauth": { "accessToken": "tok-2", "expiresAt": now_ms() + 60_000 } }),
        );
        let got = read_oauth(&settings, &api, &sign_in(&dir.join("pro")))
            .await
            .unwrap();
        assert!(
            matches!(got, Reading::Windows((Some(_), Some(_)))),
            "{got:?}"
        );
        assert_eq!(
            server.requests()[1]
                .headers
                .get("authorization")
                .map(String::as_str),
            Some("Bearer tok-2")
        );
        let none = read_oauth(&settings, &api, &sign_in(&dir.join("vide")))
            .await
            .unwrap();
        assert_eq!(none, Reading::NotSignedIn);
        // Nothing asked without a token.
        assert_eq!(server.requests().len(), 2);
        // The server refusing: an error, the last values standing.
        server.on("GET", "/api/oauth/usage", 500, json!({}));
        assert!(read_oauth(&settings, &api, &sign_in(&dir.join("pro")))
            .await
            .is_err());
    }

    #[test]
    fn the_reasons_are_told_in_both_languages() {
        use SignInProblem::*;
        assert_eq!(reason(NotSignedIn, Lang::Fr), "Pas connecté");
        assert_eq!(reason(NotSignedIn, Lang::En), "Not signed in");
        assert_eq!(
            reason(Expired, Lang::Fr),
            "Connexion expirée : relance Claude Code pour ce compte."
        );
        assert_eq!(
            reason(Expired, Lang::En),
            "Sign-in expired: run Claude Code again for this account."
        );
    }

    fn two_accounts() -> Settings {
        let mut s = Settings {
            accounts: vec![pro()],
            ..Default::default()
        };
        accounts::normalize(&mut s);
        s
    }

    fn window(pct: f64, resets_at: i64) -> Option<RateWindow> {
        Some(RateWindow {
            pct,
            resets_at: Some(resets_at),
        })
    }

    #[test]
    fn a_reading_touches_only_its_account_and_the_current_ones_windows_come_first() {
        let s = two_accounts();
        let now = now_ms();
        let hour = now + 3_600_000;
        let mut u = UsageSnapshot::default();
        u.settle(&s, now);
        let ids: Vec<&str> = u.accounts.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["principal", "pro"]);
        assert_eq!(u.current, "principal");
        // Not read yet: taken as signed in.
        assert!(u.accounts.iter().all(|a| a.connected && a.reason.is_none()));
        u.record("pro", Reading::Windows((window(40.0, hour), None)), now);
        u.settle(&s, now);
        assert_eq!(u.account("pro").unwrap().five_hour, window(40.0, hour));
        assert_eq!(u.account("pro").unwrap().updated_at, now);
        assert_eq!(u.account("principal").unwrap().five_hour, None);
        // Principal is current: its windows first, none read.
        assert_eq!((u.five_hour, u.seven_day, u.updated_at), (None, None, 0));
        // A window not told keeps its last reading.
        u.record("pro", Reading::Windows((None, window(7.0, hour))), now + 1);
        let pro = u.account("pro").unwrap();
        assert_eq!(
            (pro.five_hour, pro.seven_day, pro.updated_at),
            (window(40.0, hour), window(7.0, hour), now + 1)
        );
    }

    #[test]
    fn the_current_account_is_the_first_active_one_under_the_threshold() {
        let mut s = two_accounts();
        s.quota_pause = 90;
        let now = now_ms();
        let hour = now + 3_600_000;
        let mut u = UsageSnapshot::default();
        u.record(
            "principal",
            Reading::Windows((window(95.0, hour), None)),
            now,
        );
        u.record("pro", Reading::Windows((window(30.0, hour), None)), now);
        u.settle(&s, now);
        assert_eq!(u.current, "pro");
        assert_eq!(u.five_hour, window(30.0, hour));
        assert_eq!(u.updated_at, now);
        // Every one over it: the first active one.
        u.record("pro", Reading::Windows((None, window(92.0, hour))), now);
        u.settle(&s, now);
        assert_eq!(u.current, "principal");
        assert_eq!(u.five_hour, window(95.0, hour));
        // An inactive one is passed over.
        s.accounts[0].active = false;
        u.settle(&s, now);
        assert_eq!(u.current, "pro");
        // A window whose end has passed holds nothing back.
        s.accounts[0].active = true;
        u.record(
            "principal",
            Reading::Windows((window(95.0, now - 3_600_000), None)),
            now,
        );
        u.settle(&s, now);
        assert_eq!(u.current, "principal");
    }

    #[test]
    fn an_account_not_signed_in_or_expired_says_why_until_it_is_read_again() {
        let s = two_accounts();
        let now = now_ms();
        let mut u = UsageSnapshot::default();
        u.record(
            "pro",
            Reading::Windows((window(40.0, now + 60_000), None)),
            now,
        );
        u.record("pro", Reading::NotSignedIn, now + 1);
        u.record("principal", Reading::Expired, now + 1);
        u.settle(&s, now);
        let pro = u.account("pro").unwrap();
        assert_eq!(
            (pro.connected, pro.reason.as_deref()),
            (false, Some("Pas connecté"))
        );
        // Its last windows stay, read when they were.
        assert_eq!(
            (pro.five_hour.map(|w| w.pct), pro.updated_at),
            (Some(40.0), now)
        );
        let principal = u.account("principal").unwrap();
        assert_eq!(
            (principal.connected, principal.reason.as_deref()),
            (
                true,
                Some("Connexion expirée : relance Claude Code pour ce compte.")
            )
        );
        // Read again: signed in.
        u.record("pro", Reading::Windows((None, None)), now + 2);
        u.settle(&s, now);
        let pro = u.account("pro").unwrap();
        assert_eq!((pro.connected, pro.reason.as_deref()), (true, None));
    }

    #[test]
    fn the_accounts_follow_the_settings_order_new_and_removed_ones_included() {
        let mut s = two_accounts();
        let now = now_ms();
        let mut u = UsageSnapshot::default();
        u.record(
            "pro",
            Reading::Windows((window(40.0, now + 60_000), None)),
            now,
        );
        // Pro first, a third one added.
        s.accounts.reverse();
        s.accounts.push(Account {
            id: "perso".into(),
            ..pro()
        });
        u.settle(&s, now);
        let ids: Vec<&str> = u.accounts.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["pro", "principal", "perso"]);
        assert_eq!(u.current, "pro");
        assert_eq!(u.five_hour.map(|w| w.pct), Some(40.0));
        assert_eq!(u.account("perso").unwrap().five_hour, None);
        // Removed: gone with its readings.
        s.accounts.remove(0);
        u.settle(&s, now);
        assert!(u.account("pro").is_none());
        assert_eq!(u.current, "principal");
        assert_eq!(u.five_hour, None);
    }

    #[test]
    fn the_windows_are_kept_across_a_restart_by_account() {
        let s = two_accounts();
        let now = now_ms();
        let (five, week) = (window(80.0, now + 60_000), window(60.0, now + 120_000));
        let mut u = UsageSnapshot::default();
        u.record("principal", Reading::Windows((five, None)), now);
        u.record("pro", Reading::Windows((None, week)), now);
        u.settle(&s, now);
        let hold = Hold {
            limit_until: Some(5),
            lifted: [None, None],
            ..Default::default()
        };
        let saved = u.saved(hold.clone());
        // Principal's where the versions before the accounts read them.
        assert_eq!((saved.five_hour, saved.seven_day), (five, None));
        assert_eq!(saved.hold, hold);
        assert_eq!(
            saved.accounts,
            BTreeMap::from([(
                "pro".to_string(),
                SavedWindows {
                    five_hour: None,
                    seven_day: week
                }
            )])
        );
        let back = UsageSnapshot::restored(&saved, &s, |w| w, now);
        assert_eq!(back.account("principal").unwrap().five_hour, five);
        assert_eq!(back.account("pro").unwrap().seven_day, week);
        assert_eq!(back.current, "principal");
        assert_eq!(back.five_hour, five);
        // Only what `keep` keeps.
        let back = UsageSnapshot::restored(&saved, &s, |_| None, now);
        assert!(back
            .accounts
            .iter()
            .all(|a| a.five_hour.is_none() && a.seven_day.is_none()));
        // An account gone from the settings: its windows are not.
        let alone = Settings {
            accounts: vec![accounts::principal()],
            ..Default::default()
        };
        let back = UsageSnapshot::restored(&saved, &alone, |w| w, now);
        assert_eq!(back.accounts.len(), 1);
        // Never read: nothing kept for it.
        let mut u = UsageSnapshot::default();
        u.settle(&s, now);
        assert!(u.saved(Hold::default()).accounts.is_empty());
    }

    #[test]
    fn what_holds_the_autopilot_back_is_kept_for_the_accounts_there_are() {
        let s = two_accounts();
        let now = now_ms();
        let mut u = UsageSnapshot::default();
        u.settle(&s, now);
        let held = crate::model::AccountHold {
            limit_until: Some(9),
            ..Default::default()
        };
        let mut hold = Hold::default();
        hold.accounts.insert("pro".into(), held);
        hold.accounts.insert("parti".into(), held);
        // The pause of an account that is gone from the settings goes with it.
        let saved = u.saved(hold);
        assert_eq!(
            saved.hold.accounts,
            BTreeMap::from([("pro".to_string(), held)])
        );
    }
}
