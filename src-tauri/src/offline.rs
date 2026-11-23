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
//! switch is shown locked. It is a key of the single list of the policies (`escouade_policy::list::OFFLINE`), read by
//! `locks::Managed::read` where the 2.3 reads them all (the registry, the managed preferences of macOS, `/etc`, and
//! the e2e's file in a debug build alone, SEC-13); a team imposes it by the settings it brings back on
//! (`Locks::clamp_settings`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Serialize;

/// The policy that imposes the mode.
pub const POLICY_KEY: &str = escouade_policy::list::OFFLINE;

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

/// The policy of this device, as the managed-device policies read it (`locks::Managed`).
fn read_policy() -> bool {
    #[cfg(test)]
    if let Some(v) = *TEST_IMPOSED.lock() {
        return v;
    }
    crate::locks::Managed::read()
        .values
        .get(POLICY_KEY)
        .is_some_and(|(v, _)| truthy(v))
}

/// What the tests impose in place of the device's policies.
#[cfg(test)]
static TEST_IMPOSED: Mutex<Option<bool>> = Mutex::new(None);

#[cfg(test)]
mod tests {
    use super::*;

    /// Its own process (nextest): the state is the process's.
    #[test]
    fn the_mode_stops_the_four_flows_chosen_or_imposed() {
        *TEST_IMPOSED.lock() = Some(false);
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
        *TEST_IMPOSED.lock() = Some(true);
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
    }

    #[test]
    fn a_policy_is_a_boolean_of_its_key() {
        for (v, on) in [
            (serde_json::json!(true), true),
            (serde_json::json!(1), true),
            (serde_json::json!("true"), true),
            (serde_json::json!(0), false),
            (serde_json::json!(false), false),
        ] {
            assert_eq!(truthy(&v), on, "{v}");
        }
        // The key of the single list of the policies, which the 2.3 knows.
        assert!(escouade_policy::list::POLICIES
            .iter()
            .any(|d| d.key == POLICY_KEY));
    }
}
