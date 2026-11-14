//! The policies of a managed device (docs/TEAM-SERVER.md §6.3, TS-0b): what the IT department imposes on Escouade
//! through its own tools, with no team server.
//!
//! - **Windows**: `HKLM\SOFTWARE\Policies\Escouade`, then `HKCU\SOFTWARE\Policies\Escouade`; the machine wins
//!   ([`windows`]).
//! - **macOS**: the managed preferences domain `dev.gagnaire.escouade`, only what `CFPreferencesAppValueIsForced`
//!   says forced ([`macos`]).
//! - **Linux**: `/etc/escouade/policies/*.json`, read only from a folder and files that root alone may change
//!   ([`files`]).
//!
//! Each gives the same model: a [`Policy`] is its key, its JSON value and its [`Source`]. Nothing here ever panics on
//! what an administrator wrote: a malformed value is left out and a [`Warning`] says why. A key outside
//! [`PROVISIONAL_KEYS`] is kept and reported, until the single list of TS-12 replaces it.
//!
//! Wired into nothing yet: TS-11 (the layers and the lock) and TS-12 (the list, the ADMX, the profile) come after.
//! What was verified on each system, and what is left to TS-12's real trials: `README.md`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

pub mod files;
pub mod macos;
pub mod windows;

/// The application's identifier: its macOS preferences domain.
pub const APP_ID: &str = "dev.gagnaire.escouade";

/// The keys known so far, from the table of docs/TEAM-SERVER.md §6.4. Provisional: TS-12 makes the single list of
/// the policies, from which the ADMX, the macOS manifest and the Linux schema are generated. A key outside it is kept
/// and reported ([`Warning::UnknownKey`]).
pub const PROVISIONAL_KEYS: &[&str] = &[
    "permissions.allowBypass",
    "permissions.allowedModes",
    "permissions.defaultMode",
    "harness.allowed",
    "harness.allowedModels",
    "harness.maxEffort",
    "plugins.allowedOrigins",
    "plugins.allowed",
    "plugins.denied",
    "plugins.verifiedOnly",
    "projects.forcePrivate",
    "projects.deniedFiles",
    "proxy.url",
    "proxy.exclusions",
    "proxy.denyInsecureTls",
    "proxy.authorities",
    "telemetry.enabled",
    "voice.onlineAllowed",
    "voice.dictation",
    "remoteControl.allowed",
    "updates.channel",
    "updates.automatic",
    "servers.allowed",
    "servers.personalSpace",
    "servers.aggregateView",
    "servers.contentlessNotifications",
    "mcp.externalClients",
    "team.server",
    "team.fingerprint",
];

/// Where a policy was read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "path")]
pub enum Source {
    /// `HKLM\SOFTWARE\Policies\Escouade`: the machine's policy (Group Policy, Intune).
    Machine,
    /// `HKCU\SOFTWARE\Policies\Escouade`: the user's policy, still written by an administrator.
    User,
    /// A configuration profile on macOS (MDM): a forced value of the managed domain.
    Profile,
    /// A file of `/etc/escouade/policies`.
    File(PathBuf),
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Machine => f.write_str("the machine's policy"),
            Source::User => f.write_str("the user's policy"),
            Source::Profile => f.write_str("a configuration profile"),
            Source::File(path) => write!(f, "{}", path.display()),
        }
    }
}

/// A policy read: its key (`permissions.allowBypass`), its value and where it comes from.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Policy {
    pub key: String,
    pub value: Value,
    pub source: Source,
}

/// What was left out, or kept but is worth saying. Never fatal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Warning {
    /// A value that could not be read (wrong type, bad JSON, an invalid key): left out.
    Malformed {
        key: String,
        source: Source,
        reason: String,
    },
    /// A key outside the known list: kept.
    UnknownKey { key: String, source: Source },
    /// A whole place (a key, a folder, a file) not trusted, so not read: its owner or rights let others than an
    /// administrator change it.
    Refused { place: String, reason: String },
    /// A place that exists but could not be read.
    Unreadable { place: String, reason: String },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::Malformed {
                key,
                source,
                reason,
            } => {
                write!(f, "policy '{key}' of {source} ignored: {reason}")
            }
            Warning::UnknownKey { key, source } => {
                write!(f, "policy '{key}' of {source} is not a known key")
            }
            Warning::Refused { place, reason } => write!(f, "{place} not read: {reason}"),
            Warning::Unreadable { place, reason } => {
                write!(f, "{place} could not be read: {reason}")
            }
        }
    }
}

/// The policies of the device, one per key, and what was said while reading them.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Policies {
    policies: BTreeMap<String, Policy>,
    warnings: Vec<Warning>,
}

impl Policies {
    /// The policy of a key, if one is imposed.
    pub fn get(&self, key: &str) -> Option<&Policy> {
        self.policies.get(key)
    }

    /// The policies, by key.
    pub fn iter(&self) -> impl Iterator<Item = &Policy> {
        self.policies.values()
    }

    pub fn len(&self) -> usize {
        self.policies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }

    /// What was left out or reported, in the order it was met.
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    /// Sets a policy over any earlier one of the same key: sources are given from the weakest to the strongest.
    /// An invalid key is left out; an unknown one is kept and reported.
    pub(crate) fn set(&mut self, key: &str, value: Value, source: Source, known: &[&str]) {
        if let Err(reason) = check_key(key) {
            self.warn(Warning::Malformed {
                key: key.to_owned(),
                source,
                reason,
            });
            return;
        }
        if !known.contains(&key) {
            self.warn(Warning::UnknownKey {
                key: key.to_owned(),
                source: source.clone(),
            });
        }
        self.policies.insert(
            key.to_owned(),
            Policy {
                key: key.to_owned(),
                value,
                source,
            },
        );
    }

    pub(crate) fn warn(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }
}

/// A key is dotted words: letters, digits, `_` and `-`, separated by single dots.
fn check_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("empty key".to_owned());
    }
    if key.split('.').any(|part| part.is_empty()) {
        return Err("a key is words separated by single dots".to_owned());
    }
    if let Some(c) = key
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.'))
    {
        return Err(format!("character {c:?} not allowed in a key"));
    }
    Ok(())
}

/// A string that is a JSON object or array, the way a complex setting is written in a registry string or a
/// preference string: parsed. Any other string stays a string.
pub(crate) fn string_value(text: &str) -> Result<Value, String> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))
    } else {
        Ok(Value::String(text.to_owned()))
    }
}

/// The policies of this device, read from its system's place with the provisional list of keys.
pub fn read() -> Policies {
    #[cfg(windows)]
    {
        windows::read_system(PROVISIONAL_KEYS)
    }
    #[cfg(target_os = "macos")]
    {
        macos::read_system(PROVISIONAL_KEYS)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        files::read_system(PROVISIONAL_KEYS)
    }
    #[cfg(not(any(windows, unix)))]
    {
        Policies::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_later_source_wins_and_an_unknown_key_is_kept_and_reported() {
        let mut policies = Policies::default();
        policies.set(
            "telemetry.enabled",
            Value::Bool(true),
            Source::User,
            PROVISIONAL_KEYS,
        );
        policies.set(
            "telemetry.enabled",
            Value::Bool(false),
            Source::Machine,
            PROVISIONAL_KEYS,
        );
        policies.set(
            "some.future",
            Value::from(1),
            Source::Machine,
            PROVISIONAL_KEYS,
        );

        assert_eq!(
            policies.get("telemetry.enabled").unwrap().value,
            Value::Bool(false)
        );
        assert_eq!(
            policies.get("telemetry.enabled").unwrap().source,
            Source::Machine
        );
        assert_eq!(policies.get("some.future").unwrap().value, Value::from(1));
        assert_eq!(
            policies.warnings(),
            [Warning::UnknownKey {
                key: "some.future".into(),
                source: Source::Machine
            }]
        );
    }

    #[test]
    fn an_invalid_key_is_left_out_and_said() {
        let mut policies = Policies::default();
        for key in ["", "a..b", ".a", "a.", "a b", "é"] {
            policies.set(key, Value::Null, Source::User, PROVISIONAL_KEYS);
        }
        assert!(policies.is_empty());
        assert_eq!(policies.warnings().len(), 6);
        assert!(policies
            .warnings()
            .iter()
            .all(|w| matches!(w, Warning::Malformed { .. })));
    }

    #[test]
    fn a_string_is_json_only_when_it_opens_an_object_or_an_array() {
        assert_eq!(
            string_value("http://proxy:3128").unwrap(),
            Value::String("http://proxy:3128".into())
        );
        assert_eq!(
            string_value(r#" {"a": [1]}"#).unwrap(),
            serde_json::json!({"a": [1]})
        );
        assert_eq!(
            string_value(r#"["x","y"]"#).unwrap(),
            serde_json::json!(["x", "y"])
        );
        assert!(string_value("{not json")
            .unwrap_err()
            .starts_with("not valid JSON"));
    }

    #[test]
    fn warnings_read_as_sentences() {
        let warning = Warning::Malformed {
            key: "proxy.url".into(),
            source: Source::Machine,
            reason: "x".into(),
        };
        assert_eq!(
            warning.to_string(),
            r#"policy 'proxy.url' of the machine's policy ignored: x"#
        );
    }

    #[test]
    fn reading_this_device_never_fails() {
        // Whatever this machine holds (most likely nothing), reading gives policies and warnings, no panic.
        let _ = read();
    }
}
