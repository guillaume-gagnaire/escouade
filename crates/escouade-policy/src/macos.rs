//! macOS: the managed preferences domain `dev.gagnaire.escouade`, as a configuration profile (MDM: Jamf, Intune,
//! Kandji) sets it.
//!
//! - A value is read by `CFPreferencesCopyAppValue`, and taken as imposed only when `CFPreferencesAppValueIsForced`
//!   says it is forced, that is set by a profile. A value of `defaults write` (or the app's own preferences: the
//!   same domain holds the windows' frames) is not forced: it is ignored, silently, since it is no policy.
//! - Booleans, numbers, strings, arrays and dictionaries are JSON as they are; a string that opens an object or an
//!   array is JSON too, as on Windows. Data and dates are no policy's: said and left out.
//! - The keys asked are the known ones, the domain's key lists, and the top-level keys of the managed plists of
//!   `/Library/Managed Preferences` (whether CFPreferencesCopyKeyList lists a profile's keys is left to TS-12's
//!   trial with a real profile, README.md).
//!
//! The reading is apart from Core Foundation ([`Preferences`]), so that its tests run anywhere; the real reader
//! ([`read_system`]) is macOS only, and its own test proves that `defaults write` is not forced.

use std::collections::BTreeSet;

use serde_json::{Map, Number, Value};

use crate::{string_value, Policies, Source, Warning};

/// A preference value, as Core Foundation gives it.
#[derive(Clone, Debug, PartialEq)]
pub enum PrefValue {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<PrefValue>),
    Dictionary(Vec<(String, PrefValue)>),
    /// A type that is no policy's (data, a date), by its name.
    Unsupported(&'static str),
}

/// What a preferences domain holds, and whether each value is forced.
pub trait Preferences {
    /// The keys to ask: they may hold more than the domain has.
    fn candidate_keys(&self) -> Vec<String>;
    /// The value of a key, whatever set it.
    fn copy_value(&self, key: &str) -> Option<PrefValue>;
    /// Whether a key's value is forced: set by a configuration profile, which the user cannot change.
    fn is_forced(&self, key: &str) -> bool;
}

/// The policies of a domain: its forced values alone, under [`Source::Profile`].
pub fn read_preferences(preferences: &impl Preferences, known: &[&str]) -> Policies {
    let mut policies = Policies::default();
    let keys: BTreeSet<String> = preferences.candidate_keys().into_iter().collect();
    for key in keys {
        if !preferences.is_forced(&key) {
            continue;
        }
        let Some(value) = preferences.copy_value(&key) else {
            continue;
        };
        match json_of(&value, true) {
            Ok(json) => policies.set(&key, json, Source::Profile, known),
            Err(reason) => policies.warn(Warning::Malformed {
                key,
                source: Source::Profile,
                reason,
            }),
        }
    }
    policies
}

/// A preference as JSON. At the top, a string may be JSON (a complex setting); inside an array or a dictionary, a
/// string is a string.
fn json_of(value: &PrefValue, top: bool) -> Result<Value, String> {
    Ok(match value {
        PrefValue::Bool(b) => Value::Bool(*b),
        PrefValue::Integer(n) => Value::from(*n),
        PrefValue::Float(x) => {
            Value::Number(Number::from_f64(*x).ok_or_else(|| format!("{x} is not a JSON number"))?)
        }
        PrefValue::String(s) if top => string_value(s)?,
        PrefValue::String(s) => Value::String(s.clone()),
        PrefValue::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| json_of(item, false))
                .collect::<Result<_, _>>()?,
        ),
        PrefValue::Dictionary(entries) => {
            let mut map = Map::new();
            for (k, v) in entries {
                map.insert(k.clone(), json_of(v, false)?);
            }
            Value::Object(map)
        }
        PrefValue::Unsupported(kind) => return Err(format!("a {kind} is not a policy's value")),
    })
}

#[cfg(target_os = "macos")]
pub use system::*;

#[cfg(target_os = "macos")]
mod system {
    use std::path::Path;

    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::data::CFData;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::propertylist::create_with_data;
    use core_foundation::string::CFString;
    use core_foundation_sys::base::CFTypeRef;
    use core_foundation_sys::number::CFNumberIsFloatType;
    use core_foundation_sys::preferences::{
        kCFPreferencesAnyHost, kCFPreferencesAnyUser, kCFPreferencesCurrentHost,
        kCFPreferencesCurrentUser, CFPreferencesAppSynchronize, CFPreferencesAppValueIsForced,
        CFPreferencesCopyAppValue, CFPreferencesCopyKeyList,
    };
    use core_foundation_sys::propertylist::kCFPropertyListImmutable;

    use super::{read_preferences, PrefValue, Preferences};
    use crate::{Policies, APP_ID};

    /// Where the profiles' preferences are written, for the machine and for each user.
    const MANAGED: &str = "/Library/Managed Preferences";

    /// A preferences domain of this Mac, read through CFPreferences.
    pub struct SystemPreferences {
        app: CFString,
        domain: String,
        known: Vec<String>,
    }

    impl SystemPreferences {
        pub fn new(domain: &str, known: &[&str]) -> SystemPreferences {
            let app = CFString::new(domain);
            // SAFETY: a valid application id; synchronizing reads again what cfprefsd holds.
            unsafe { CFPreferencesAppSynchronize(app.as_concrete_TypeRef()) };
            SystemPreferences {
                app,
                domain: domain.to_owned(),
                known: known.iter().map(|k| k.to_string()).collect(),
            }
        }
    }

    impl Preferences for SystemPreferences {
        fn candidate_keys(&self) -> Vec<String> {
            let mut keys = self.known.clone();
            // SAFETY: CF constants of the framework.
            let scopes = unsafe {
                [
                    (kCFPreferencesCurrentUser, kCFPreferencesAnyHost),
                    (kCFPreferencesCurrentUser, kCFPreferencesCurrentHost),
                    (kCFPreferencesAnyUser, kCFPreferencesAnyHost),
                    (kCFPreferencesAnyUser, kCFPreferencesCurrentHost),
                ]
            };
            for (user, host) in scopes {
                // SAFETY: a valid application id and CF constants; the array is ours (create rule).
                let list =
                    unsafe { CFPreferencesCopyKeyList(self.app.as_concrete_TypeRef(), user, host) };
                if list.is_null() {
                    continue;
                }
                // SAFETY: a CFArray we own.
                let list: CFArray<*const std::ffi::c_void> =
                    unsafe { CFArray::wrap_under_create_rule(list) };
                keys.extend(
                    list.get_all_values()
                        .into_iter()
                        .filter_map(|item| string_of(item as CFTypeRef)),
                );
            }
            let mut plists = vec![Path::new(MANAGED).join(format!("{}.plist", self.domain))];
            if let Ok(user) = std::env::var("USER") {
                plists.push(
                    Path::new(MANAGED)
                        .join(user)
                        .join(format!("{}.plist", self.domain)),
                );
            }
            for plist in plists {
                keys.extend(plist_keys(&plist));
            }
            keys
        }

        fn copy_value(&self, key: &str) -> Option<PrefValue> {
            let key = CFString::new(key);
            // SAFETY: valid strings; the value is ours (create rule), or null.
            let value = unsafe {
                CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), self.app.as_concrete_TypeRef())
            };
            if value.is_null() {
                return None;
            }
            // SAFETY: a CF object we own.
            let value = unsafe { CFType::wrap_under_create_rule(value) };
            Some(pref_of(&value))
        }

        fn is_forced(&self, key: &str) -> bool {
            let key = CFString::new(key);
            // SAFETY: valid strings.
            unsafe {
                CFPreferencesAppValueIsForced(
                    key.as_concrete_TypeRef(),
                    self.app.as_concrete_TypeRef(),
                ) != 0
            }
        }
    }

    /// The policies of this Mac: the forced values of `dev.gagnaire.escouade`.
    pub fn read_system(known: &[&str]) -> Policies {
        read_domain(APP_ID, known)
    }

    /// The policies of a domain (the tests use their own).
    pub fn read_domain(domain: &str, known: &[&str]) -> Policies {
        read_preferences(&SystemPreferences::new(domain, known), known)
    }

    fn string_of(item: CFTypeRef) -> Option<String> {
        // SAFETY: an item of a CF collection, retained by the wrapper for as long as it lives.
        let item = unsafe { CFType::wrap_under_get_rule(item) };
        item.downcast::<CFString>().map(|s| s.to_string())
    }

    /// The top-level keys of a plist file, if it is a dictionary.
    fn plist_keys(path: &Path) -> Vec<String> {
        let Ok(bytes) = std::fs::read(path) else {
            return Vec::new();
        };
        let Ok((list, _)) = create_with_data(CFData::from_buffer(&bytes), kCFPropertyListImmutable)
        else {
            return Vec::new();
        };
        // SAFETY: a property list we own (create rule).
        let list = unsafe { CFType::wrap_under_create_rule(list as CFTypeRef) };
        let Some(dictionary) = list.downcast::<CFDictionary>() else {
            return Vec::new();
        };
        let (keys, _) = dictionary.get_keys_and_values();
        keys.into_iter()
            .filter_map(|key| string_of(key as CFTypeRef))
            .collect()
    }

    fn pref_of(value: &CFType) -> PrefValue {
        if let Some(b) = value.downcast::<CFBoolean>() {
            return PrefValue::Bool(b.into());
        }
        if let Some(n) = value.downcast::<CFNumber>() {
            // SAFETY: a CFNumber.
            let float = unsafe { CFNumberIsFloatType(n.as_concrete_TypeRef()) } != 0;
            return if float {
                n.to_f64()
                    .map(PrefValue::Float)
                    .unwrap_or(PrefValue::Unsupported("number"))
            } else {
                n.to_i64()
                    .map(PrefValue::Integer)
                    .unwrap_or(PrefValue::Unsupported("number"))
            };
        }
        if let Some(s) = value.downcast::<CFString>() {
            return PrefValue::String(s.to_string());
        }
        if let Some(array) = value.downcast::<CFArray>() {
            return PrefValue::Array(
                array
                    .get_all_values()
                    .into_iter()
                    // SAFETY: an item of the array, retained by the wrapper.
                    .map(|item| pref_of(&unsafe { CFType::wrap_under_get_rule(item as CFTypeRef) }))
                    .collect(),
            );
        }
        if let Some(dictionary) = value.downcast::<CFDictionary>() {
            let (keys, values) = dictionary.get_keys_and_values();
            let mut entries = Vec::new();
            for (key, value) in keys.into_iter().zip(values) {
                let Some(key) = string_of(key as CFTypeRef) else {
                    return PrefValue::Unsupported("dictionary with keys that are not strings");
                };
                // SAFETY: a value of the dictionary, retained by the wrapper.
                entries.push((
                    key,
                    pref_of(&unsafe { CFType::wrap_under_get_rule(value as CFTypeRef) }),
                ));
            }
            return PrefValue::Dictionary(entries);
        }
        PrefValue::Unsupported("value of another type (data, date)")
    }

    #[cfg(test)]
    mod tests {
        use std::process::Command;

        use super::*;
        use crate::PROVISIONAL_KEYS;

        /// A test domain of `defaults`, deleted when dropped.
        struct TestDomain(String);

        impl Drop for TestDomain {
            fn drop(&mut self) {
                let _ = Command::new("defaults").args(["delete", &self.0]).status();
            }
        }

        fn defaults_write(domain: &str, args: &[&str]) {
            let status = Command::new("defaults")
                .arg("write")
                .arg(domain)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        }

        #[test]
        fn a_value_of_defaults_write_is_read_but_not_forced_so_no_policy() {
            let domain = TestDomain(format!(
                "dev.gagnaire.escouade.policytest{}",
                std::process::id()
            ));
            defaults_write(&domain.0, &["telemetry.enabled", "-bool", "false"]);
            defaults_write(
                &domain.0,
                &["proxy.url", "-string", "http://proxy.acme:3128"],
            );
            defaults_write(&domain.0, &["harness.allowed", "-array", "claude", "codex"]);

            let preferences = SystemPreferences::new(&domain.0, PROVISIONAL_KEYS);
            // CFPreferences sees the values: the reader does reach them…
            assert_eq!(
                preferences.copy_value("telemetry.enabled"),
                Some(PrefValue::Bool(false))
            );
            assert_eq!(
                preferences.copy_value("harness.allowed"),
                Some(PrefValue::Array(vec![
                    PrefValue::String("claude".into()),
                    PrefValue::String("codex".into())
                ]))
            );
            assert!(preferences
                .candidate_keys()
                .iter()
                .any(|k| k == "proxy.url"));
            // …but none is forced, so none is a policy.
            assert!(!preferences.is_forced("telemetry.enabled"));
            let policies = read_domain(&domain.0, PROVISIONAL_KEYS);
            assert!(policies.is_empty(), "{policies:?}");
            assert!(policies.warnings().is_empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;
    use crate::PROVISIONAL_KEYS;

    /// Preferences in memory: a value, and whether it is forced.
    struct Fake(HashMap<&'static str, (PrefValue, bool)>);

    impl Preferences for Fake {
        fn candidate_keys(&self) -> Vec<String> {
            let mut keys: Vec<String> = self.0.keys().map(|k| k.to_string()).collect();
            keys.push("telemetry.enabled".into()); // asked twice: read once
            keys.push("absent.key".into());
            keys
        }
        fn copy_value(&self, key: &str) -> Option<PrefValue> {
            self.0.get(key).map(|(v, _)| v.clone())
        }
        fn is_forced(&self, key: &str) -> bool {
            self.0.get(key).is_some_and(|(_, f)| *f)
        }
    }

    #[test]
    fn only_forced_values_are_policies() {
        let fake = Fake(HashMap::from([
            ("telemetry.enabled", (PrefValue::Bool(false), true)),
            ("voice.dictation", (PrefValue::Bool(true), false)),
            (
                "NSWindow Frame main",
                (PrefValue::String("0 0 800 600".into()), false),
            ),
            ("harness.maxEffort", (PrefValue::Integer(2), true)),
            ("proxy.url", (PrefValue::String("http://p:1".into()), true)),
            (
                "harness.allowedModels",
                (PrefValue::String(r#"{"claude":["opus"]}"#.into()), true),
            ),
            (
                "plugins.allowedOrigins",
                (
                    PrefValue::Array(vec![
                        PrefValue::String("[store]".into()),
                        PrefValue::Float(1.5),
                    ]),
                    true,
                ),
            ),
            (
                "servers.allowed",
                (
                    PrefValue::Dictionary(vec![("acme".into(), PrefValue::Bool(true))]),
                    true,
                ),
            ),
        ]));
        let policies = read_preferences(&fake, PROVISIONAL_KEYS);
        let value = |key: &str| policies.get(key).map(|p| p.value.clone());
        assert_eq!(policies.len(), 6);
        assert!(policies.iter().all(|p| p.source == Source::Profile));
        assert_eq!(value("telemetry.enabled"), Some(json!(false)));
        assert_eq!(value("voice.dictation"), None);
        assert_eq!(value("harness.maxEffort"), Some(json!(2)));
        assert_eq!(value("proxy.url"), Some(json!("http://p:1")));
        assert_eq!(
            value("harness.allowedModels"),
            Some(json!({"claude": ["opus"]}))
        );
        // Inside an array, a string that looks like JSON stays a string.
        assert_eq!(
            value("plugins.allowedOrigins"),
            Some(json!(["[store]", 1.5]))
        );
        assert_eq!(value("servers.allowed"), Some(json!({"acme": true})));
        assert!(policies.warnings().is_empty(), "{:?}", policies.warnings());
    }

    #[test]
    fn malformed_forced_values_are_left_out_and_said() {
        let fake = Fake(HashMap::from([
            ("proxy.authorities", (PrefValue::Unsupported("data"), true)),
            (
                "harness.allowedModels",
                (PrefValue::String("{oops".into()), true),
            ),
            (
                "updates.channel",
                (PrefValue::Array(vec![PrefValue::Float(f64::NAN)]), true),
            ),
            ("bad key", (PrefValue::Bool(true), true)),
            ("telemetry.enabled", (PrefValue::Bool(false), true)),
        ]));
        let policies = read_preferences(&fake, PROVISIONAL_KEYS);
        assert_eq!(policies.len(), 1);
        assert_eq!(policies.warnings().len(), 4, "{:?}", policies.warnings());
        assert!(policies
            .warnings()
            .iter()
            .all(|w| matches!(w, Warning::Malformed { .. })));
    }

    #[test]
    fn an_unknown_forced_key_is_kept_and_reported() {
        let fake = Fake(HashMap::from([(
            "future.policy",
            (PrefValue::Integer(1), true),
        )]));
        let policies = read_preferences(&fake, PROVISIONAL_KEYS);
        assert_eq!(policies.get("future.policy").unwrap().value, json!(1));
        assert!(matches!(policies.warnings(), [Warning::UnknownKey { .. }]));
    }
}
