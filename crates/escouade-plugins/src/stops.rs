//! What is told when a plugin stops or another version of it comes: what the app keeps of a plugin beside the host
//! (the instance of its terminal views, `plugins::view`) asks it here when it starts keeping it.

use parking_lot::Mutex;
use std::collections::HashMap;

/// What to tell of each plugin, by id.
type Told = HashMap<String, fn(&str)>;
static TOLD: Mutex<Option<Told>> = Mutex::new(None);

/// `forget` is told when `plugin` stops (each time, until the end of the app).
pub fn on_stop(plugin: &str, forget: fn(&str)) {
    TOLD.lock()
        .get_or_insert_with(HashMap::new)
        .insert(plugin.to_string(), forget);
}

/// `plugin` stops: what asked it is told (nothing, for a plugin nothing is kept of).
pub fn stopped(plugin: &str) {
    let told = TOLD.lock().as_ref().and_then(|m| m.get(plugin).copied());
    if let Some(forget) = told {
        forget(plugin);
    }
}
