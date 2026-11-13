//! The ports the shared processes of the harnesses said they listen on (the app's `plugins::shared` runs them and notes
//! each here as it keeps it): all a plugin's `local:shared` may reach (`Runtime`). Never a port the plugin named.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;

/// A process kept, by its key (`<runtime>|<plugin>|…`): the port it said (0 before and without), whether it exited.
type Noted = HashMap<String, (Arc<AtomicU16>, Arc<AtomicBool>)>;
static NOTED: Mutex<Option<Noted>> = Mutex::new(None);

/// The process of `key` is kept, in place of any before it.
pub fn note(key: &str, port: Arc<AtomicU16>, exited: Arc<AtomicBool>) {
    NOTED
        .lock()
        .get_or_insert_with(HashMap::new)
        .insert(key.to_string(), (port, exited));
}

/// The process of `key` whose port is `port` is no longer kept (another of the same key stays).
pub fn forget(key: &str, port: &Arc<AtomicU16>) {
    let mut all = NOTED.lock();
    let Some(map) = all.as_mut() else { return };
    if map.get(key).is_some_and(|(p, _)| Arc::ptr_eq(p, port)) {
        map.remove(key);
    }
}

/// The ports the processes of `plugin` in the runtime `rt` said they listen on, while they live.
pub fn ports_of(rt: usize, plugin: &str) -> Vec<u16> {
    let prefix = format!("{rt}|{plugin}|");
    NOTED.lock().as_ref().map_or(Vec::new(), |map| {
        map.iter()
            .filter(|(key, (_, exited))| {
                !exited.load(Ordering::Acquire) && key.starts_with(&prefix)
            })
            .map(|(_, (port, _))| port.load(Ordering::Acquire))
            .filter(|p| *p != 0)
            .collect()
    })
}
