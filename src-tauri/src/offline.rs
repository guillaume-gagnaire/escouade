//! « Mode hors ligne » (audit SEC-18, ESC-596): one gesture stops the four flows the app starts by itself, which a
//! firewall would otherwise have to know one by one:
//!
//! - the updates: the app's (`update_check`, every 5 minutes) and the plugins' (looked for at the start);
//! - the store's catalogue, read at the start and every 6 hours;
//! - the revocation list, read with it, and its attestation of freshness;
//! - the usage statistics (Aptabase), which leave every minute after a yes: none is kept while it lasts.
//!
//! What the user asks by a gesture of its own (install a plugin, show an image, an agent's work, `git fetch`) is not
//! one of them. The core refuses them itself, whatever the window asks; a click on « Rechercher des mises à jour » or
//! « Relire » says that the mode is on.
//!
//! **Imposed by a managed device.** The policy `network.offline` (a Boolean) turns it on whatever Réglages say, and the
//! switch is shown locked. It is read where the managed-device policies of the 2.3 read them (docs/TEAM-SERVER.md §6.3):
//! `HKLM\SOFTWARE\Policies\Escouade`, then `HKCU\…` (a `REG_DWORD`, 1), the managed preferences of macOS
//! (`/Library/Managed Preferences/[<user>/]dev.gagnaire.escouade.plist`), and `/etc/escouade/policies/*.json` owned by
//! root and written by root alone. In a debug build, `ESCOUADE_TEST_POLICIES` names a JSON file read in their place
//! (the tests'); a release build never reads it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Serialize;

/// The policy that imposes the mode.
pub const POLICY_KEY: &str = "network.offline";

/// What the user chose in Réglages, as the core holds it.
static CHOSEN: AtomicBool = AtomicBool::new(false);

/// The policy as last read, and when: read again after a minute, so that a policy put while the app runs applies.
static IMPOSED: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
const READ_AGAIN: Duration = Duration::from_secs(60);

/// The mode, as the window shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// The four flows are stopped.
    pub on: bool,
    /// By a policy of this device: the switch is locked.
    pub imposed: bool,
}

/// The four flows, and whether each may go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Updates,
    Store,
    Revocations,
    Statistics,
}

/// The four, for the tests.
#[cfg_attr(not(test), allow(dead_code))]
pub const FLOWS: [Flow; 4] = [
    Flow::Updates,
    Flow::Store,
    Flow::Revocations,
    Flow::Statistics,
];

/// What the user chose, from the settings saved or read.
pub fn set_chosen(on: bool) {
    CHOSEN.store(on, Ordering::SeqCst);
}

/// Whether a policy of this device imposes the mode.
pub fn imposed() -> bool {
    let mut held = IMPOSED.lock();
    if let Some((at, v)) = *held {
        if at.elapsed() < READ_AGAIN {
            return v;
        }
    }
    let v = read_policy();
    *held = Some((Instant::now(), v));
    v
}

/// Forgets the policy read, for the next call to read it again (the tests', and a gesture of the user).
pub fn read_again() {
    *IMPOSED.lock() = None;
}

/// The mode now.
pub fn state() -> State {
    let imposed = imposed();
    State {
        on: imposed || CHOSEN.load(Ordering::SeqCst),
        imposed,
    }
}

/// Whether `flow` may go now: none of the four while the mode is on.
pub fn allows(_flow: Flow) -> bool {
    !state().on
}

/// The words of a flow refused, for a gesture of the user.
pub fn refusal(lang: crate::i18n::Lang) -> String {
    tr_in!(
        lang,
        "Mode hors ligne : Escouade ne contacte ni les mises à jour, ni le store, ni la liste de révocation, ni les statistiques.",
        "Offline mode: Escouade contacts neither the updates, nor the store, nor the revocation list, nor the statistics."
    )
}

/// A policy's value read as a Boolean: `true`, `1`, `"true"`.
fn truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_u64().is_some_and(|n| n != 0),
        serde_json::Value::String(s) => matches!(s.trim(), "1" | "true"),
        _ => false,
    }
}

/// `network.offline` of a JSON object of policies (`/etc`, the tests' file).
pub fn from_json(text: &str) -> Option<bool> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    v.as_object()?.get(POLICY_KEY).map(truthy)
}

fn read_policy() -> bool {
    if cfg!(debug_assertions) {
        if let Some(path) = std::env::var_os("ESCOUADE_TEST_POLICIES").filter(|p| !p.is_empty()) {
            return std::fs::read_to_string(path)
                .ok()
                .and_then(|t| from_json(&t))
                .unwrap_or(false);
        }
    }
    system_policy()
}

#[cfg(windows)]
fn system_policy() -> bool {
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD,
    };
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (path, name) = (wide(r"SOFTWARE\Policies\Escouade"), wide(POLICY_KEY));
    // The machine first: it wins over the user.
    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let mut value: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: the names end with a nul, and the value is a u32 of the size given.
        let r = unsafe {
            RegGetValueW(
                root,
                path.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut value as *mut u32).cast(),
                &mut size,
            )
        };
        if r == 0 {
            return value != 0;
        }
    }
    false
}

#[cfg(target_os = "macos")]
fn system_policy() -> bool {
    // What a configuration profile of an MDM wrote: for the user, then for the machine.
    let user = std::env::var("USER").unwrap_or_default();
    let mut places = Vec::new();
    if !user.is_empty() && !user.contains(['/', '.']) {
        places.push(format!(
            "/Library/Managed Preferences/{user}/dev.gagnaire.escouade.plist"
        ));
    }
    places.push("/Library/Managed Preferences/dev.gagnaire.escouade.plist".into());
    places.iter().find_map(|p| {
        let v: plist::Value = plist::from_file(p).ok()?;
        let v = v.as_dictionary()?.get(POLICY_KEY)?;
        Some(
            v.as_boolean()
                .or_else(|| v.as_unsigned_integer().map(|n| n != 0))
                .unwrap_or(false),
        )
    }) == Some(true)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn system_policy() -> bool {
    use std::os::unix::fs::MetadataExt;
    let dir = std::path::Path::new("/etc/escouade/policies");
    // Root's alone: owned by root, written by neither its group nor the others.
    let root_only = |m: &std::fs::Metadata| m.uid() == 0 && m.mode() & 0o022 == 0;
    if !std::fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir() && root_only(&m)) {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut files: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    // The last file that says it wins.
    files
        .iter()
        .filter(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file() && root_only(&m)))
        .filter_map(|p| std::fs::read_to_string(p).ok().and_then(|t| from_json(&t)))
        .last()
        .unwrap_or(false)
}

#[cfg(not(any(windows, unix)))]
fn system_policy() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Its own process (nextest): the state is the process's.
    #[test]
    fn the_mode_stops_the_four_flows_chosen_or_imposed() {
        let dir = std::env::temp_dir().join(format!("offline-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("policies.json");
        std::fs::write(&file, r#"{"network.offline": false}"#).unwrap();
        std::env::set_var("ESCOUADE_TEST_POLICIES", &file);
        read_again();
        set_chosen(false);
        assert_eq!(
            state(),
            State {
                on: false,
                imposed: false
            }
        );
        assert!(FLOWS.iter().all(|f| allows(*f)));
        // Chosen in Réglages.
        set_chosen(true);
        assert_eq!(
            state(),
            State {
                on: true,
                imposed: false
            }
        );
        assert!(FLOWS.iter().all(|f| !allows(*f)));
        // Imposed by the device: on whatever the user chose.
        std::fs::write(&file, r#"{"network.offline": true}"#).unwrap();
        read_again();
        set_chosen(false);
        assert_eq!(
            state(),
            State {
                on: true,
                imposed: true
            }
        );
        assert!(FLOWS.iter().all(|f| !allows(*f)));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_policy_is_a_boolean_of_its_key() {
        assert_eq!(from_json(r#"{"network.offline": true}"#), Some(true));
        assert_eq!(from_json(r#"{"network.offline": 1}"#), Some(true));
        assert_eq!(from_json(r#"{"network.offline": "true"}"#), Some(true));
        assert_eq!(from_json(r#"{"network.offline": 0}"#), Some(false));
        assert_eq!(from_json(r#"{"telemetry.enabled": false}"#), None);
        assert_eq!(from_json("not json"), None);
        assert_eq!(from_json("[true]"), None);
    }
}
