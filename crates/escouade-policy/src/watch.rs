//! The policies read again while the app runs (docs/TEAM-SERVER.md §6.3, TS-12): a policy an administrator changes
//! applies without a restart.
//!
//! A thread waits for a sign that something changed ([`Changes`]), reads the policies again, and calls back when they
//! differ from the last ones. The signs:
//! - **Windows**: `RegNotifyChangeKeyValue` on `HKLM\SOFTWARE\Policies` and `HKCU\SOFTWARE\Policies`, their whole
//!   subtree (the `Escouade` key may not exist yet: Group Policy creates it);
//! - **macOS**: the files of `/Library/Managed Preferences`, which the profiles write;
//! - **Linux**: the files of `/etc/escouade`.
//!
//! The files are watched by their names, sizes and dates, looked at every second ([`Poll`]): no dependency, and a
//! missing folder that appears later is seen. Whatever the sign, the policies are read again every `period` too, in case
//! a change gave none (a preference cached by macOS). Tests give their own sign ([`Channel`]) and their own reading.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use crate::Policies;

/// A sign that the policies may have changed.
pub trait Changes: Send {
    /// Waits for a sign, `timeout` at most; true when one came.
    fn wait(&mut self, timeout: Duration) -> bool;
}

/// A watch running: it stops when dropped (at its next wake).
pub struct Watch {
    stop: Arc<AtomicBool>,
}

impl Watch {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Watches the policies: `read` reads them, `changes` signs a change, `last` are those already applied. `on_change` is
/// called with the new ones each time they differ, on the watch's own thread.
pub fn spawn<F, C>(
    mut read: F,
    mut changes: Box<dyn Changes>,
    last: Policies,
    period: Duration,
    mut on_change: C,
) -> Watch
where
    F: FnMut() -> Policies + Send + 'static,
    C: FnMut(Policies) + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let _ = std::thread::Builder::new()
        .name("policies-watch".into())
        .spawn(move || {
            let mut last = last;
            while !stopped.load(Ordering::Acquire) {
                changes.wait(period);
                if stopped.load(Ordering::Acquire) {
                    break;
                }
                let now = read();
                if now != last {
                    last = now.clone();
                    on_change(now);
                }
            }
        });
    Watch { stop }
}

/// The sign a test gives: a message on a channel.
pub struct Channel(pub std::sync::mpsc::Receiver<()>);

impl Changes for Channel {
    fn wait(&mut self, timeout: Duration) -> bool {
        let got = self.0.recv_timeout(timeout).is_ok();
        // Several signs at once are one.
        while self.0.try_recv().is_ok() {}
        got
    }
}

/// The files under some folders, by their names, sizes and dates: a change of one of them is a sign.
pub struct Poll {
    roots: Vec<PathBuf>,
    every: Duration,
    last: Vec<(PathBuf, u64, Option<SystemTime>)>,
}

impl Poll {
    pub fn new(roots: Vec<PathBuf>, every: Duration) -> Poll {
        let last = snapshot(&roots);
        Poll { roots, every, last }
    }
}

/// The files under `roots`, two levels down (a profile per user is in a folder of its own), sorted.
fn snapshot(roots: &[PathBuf]) -> Vec<(PathBuf, u64, Option<SystemTime>)> {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<(PathBuf, u64, Option<SystemTime>)>) {
        let Ok(meta) = std::fs::symlink_metadata(dir) else {
            return;
        };
        out.push((dir.to_owned(), meta.len(), meta.modified().ok()));
        if !meta.is_dir() || depth == 0 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            walk(&e.path(), depth - 1, out);
        }
    }
    let mut out = Vec::new();
    for root in roots {
        walk(root, 2, &mut out);
    }
    out.sort();
    out
}

impl Changes for Poll {
    fn wait(&mut self, timeout: Duration) -> bool {
        let end = Instant::now() + timeout;
        loop {
            let now = snapshot(&self.roots);
            if now != self.last {
                self.last = now;
                return true;
            }
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            std::thread::sleep(left.min(self.every));
        }
    }
}

/// The signs of this system's place of the policies.
pub fn system() -> Box<dyn Changes> {
    #[cfg(windows)]
    {
        Box::new(registry::Registry::policies())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(Poll::new(
            vec![PathBuf::from("/Library/Managed Preferences")],
            Duration::from_secs(1),
        ))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Box::new(Poll::new(
            vec![PathBuf::from("/etc/escouade")],
            Duration::from_secs(1),
        ))
    }
    #[cfg(not(any(windows, unix)))]
    {
        Box::new(Poll::new(Vec::new(), Duration::from_secs(1)))
    }
}

/// Watches the policies of this device: [`crate::read`] again at each sign of [`system`], and every `period`.
pub fn system_watch<C>(last: Policies, period: Duration, on_change: C) -> Watch
where
    C: FnMut(Policies) + Send + 'static,
{
    spawn(crate::read, system(), last, period, on_change)
}

#[cfg(windows)]
pub mod registry {
    //! `RegNotifyChangeKeyValue`: the registry tells of a change under a key, once per registration.

    use std::time::Duration;

    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
        HKEY_LOCAL_MACHINE, KEY_NOTIFY, KEY_WOW64_64KEY, REG_NOTIFY_CHANGE_LAST_SET,
        REG_NOTIFY_CHANGE_NAME,
    };
    use windows_sys::Win32::System::Threading::{CreateEventW, WaitForMultipleObjects};

    use super::Changes;

    struct Watched {
        key: HKEY,
        event: HANDLE,
        armed: bool,
    }

    /// Keys of the registry watched, with their subtrees.
    pub struct Registry {
        keys: Vec<Watched>,
    }

    // The handles are the thread's that waits on them; they move with it.
    unsafe impl Send for Registry {}

    impl Registry {
        /// `HKLM\SOFTWARE\Policies` and `HKCU\SOFTWARE\Policies`.
        pub fn policies() -> Registry {
            Registry::of(&[
                (HKEY_LOCAL_MACHINE, r"SOFTWARE\Policies"),
                (HKEY_CURRENT_USER, r"SOFTWARE\Policies"),
            ])
        }

        /// Keys under their hive, by their paths; one that cannot be opened is left out (the period still reads).
        pub fn of(keys: &[(HKEY, &str)]) -> Registry {
            let mut watched = Vec::new();
            for (hive, path) in keys {
                let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
                let mut key: HKEY = std::ptr::null_mut();
                // SAFETY: a valid hive, a NUL-ended path, an out pointer to a key of ours.
                let opened = unsafe {
                    RegOpenKeyExW(
                        *hive,
                        wide.as_ptr(),
                        0,
                        KEY_NOTIFY | KEY_WOW64_64KEY,
                        &mut key,
                    )
                };
                if opened != ERROR_SUCCESS {
                    continue;
                }
                // SAFETY: an auto-reset event, unnamed, not signalled.
                let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
                if event.is_null() {
                    // SAFETY: the key just opened.
                    unsafe { RegCloseKey(key) };
                    continue;
                }
                watched.push(Watched {
                    key,
                    event,
                    armed: false,
                });
            }
            Registry { keys: watched }
        }

        /// How many keys are watched.
        pub fn len(&self) -> usize {
            self.keys.len()
        }

        pub fn is_empty(&self) -> bool {
            self.keys.is_empty()
        }
    }

    impl Changes for Registry {
        fn wait(&mut self, timeout: Duration) -> bool {
            for w in &mut self.keys {
                if !w.armed {
                    // SAFETY: a key and an event of ours; asynchronous, so the call returns at once.
                    let r = unsafe {
                        RegNotifyChangeKeyValue(
                            w.key,
                            1,
                            REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                            w.event,
                            1,
                        )
                    };
                    w.armed = r == ERROR_SUCCESS;
                }
            }
            if self.keys.is_empty() {
                std::thread::sleep(timeout);
                return false;
            }
            let events: Vec<HANDLE> = self.keys.iter().map(|w| w.event).collect();
            let ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX - 1);
            // SAFETY: events of ours, alive for the call.
            let r = unsafe { WaitForMultipleObjects(events.len() as u32, events.as_ptr(), 0, ms) };
            let i = r.wrapping_sub(WAIT_OBJECT_0) as usize;
            if i < self.keys.len() {
                // Told once: armed again at the next wait.
                self.keys[i].armed = false;
                true
            } else {
                false
            }
        }
    }

    impl Drop for Registry {
        fn drop(&mut self) {
            for w in &self.keys {
                // SAFETY: handles of ours, closed once.
                unsafe {
                    RegCloseKey(w.key);
                    CloseHandle(w.event);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
