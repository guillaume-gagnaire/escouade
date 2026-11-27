//! Commands of the tests alone (`#[cfg(test)]`, found by [`super::find`] after the list): calls that never give their
//! hand back (ESC-654), to show what the server does of them when their connection closes or their device is revoked.
//! Each keeps what it saw under the key its call gives (`key`), so that tests running together never read each other's.

use super::{Call, Class, Command, Reply, Scope};
use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;
use tauri::ipc::InvokeResponseBody;

/// What the calls of a key did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    /// `__test_hang` started, and dropped (its future let go by the server).
    pub hang_started: usize,
    pub hang_dropped: usize,
    /// `__test_hold` ran (its blocking job started).
    pub hold_ran: usize,
}

static SEEN: LazyLock<Mutex<HashMap<String, Seen>>> = LazyLock::new(Default::default);
/// The keys whose `__test_hold` may end.
static RELEASED: LazyLock<(Mutex<Vec<String>>, Condvar)> = LazyLock::new(Default::default);

pub const COMMANDS: &[Command] = &[
    Command {
        name: "__test_hang",
        class: Class::Served,
        scope: Scope::Phone,
        call: hang,
    },
    Command {
        name: "__test_hold",
        class: Class::Served,
        scope: Scope::Phone,
        call: hold,
    },
];

/// What the calls of `key` did so far.
pub fn seen(key: &str) -> Seen {
    SEEN.lock().get(key).copied().unwrap_or_default()
}

/// The `__test_hold` of `key` may end.
pub fn release(key: &str) {
    let (released, cv) = &*RELEASED;
    released.lock().push(key.to_string());
    cv.notify_all();
}

fn key_of(call: &Call) -> String {
    call.args
        .get("key")
        .and_then(|k| k.as_str())
        .unwrap_or("")
        .to_string()
}

fn note(key: &str, f: impl FnOnce(&mut Seen)) {
    f(SEEN.lock().entry(key.to_string()).or_default());
}

/// A future that never ends, its drop noted.
fn hang(call: Call) -> Reply {
    struct Dropped(String);
    impl Drop for Dropped {
        fn drop(&mut self) {
            note(&self.0, |s| s.hang_dropped += 1);
        }
    }
    let key = key_of(&call);
    Reply::future(async move {
        note(&key, |s| s.hang_started += 1);
        let _dropped = Dropped(key);
        std::future::pending::<()>().await;
        Ok(InvokeResponseBody::Json("null".into()))
    })
}

/// A blocking job that holds its thread (and its lane) until its key is released, 60 s at most.
fn hold(call: Call) -> Reply {
    let key = key_of(&call);
    Reply::blocking(move || {
        note(&key, |s| s.hold_ran += 1);
        let (released, cv) = &*RELEASED;
        let mut released = released.lock();
        let until = std::time::Instant::now() + Duration::from_secs(60);
        while !released.contains(&key) && std::time::Instant::now() < until {
            cv.wait_for(&mut released, Duration::from_millis(200));
        }
        Ok(InvokeResponseBody::Json("null".into()))
    })
}
