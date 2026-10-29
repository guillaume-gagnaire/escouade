//! Single event channel towards the webview.

use crate::model::UiEvent;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::mem::Discriminant;
use std::time::{Duration, Instant};
use tauri::ipc::Channel;

/// Past this many events sent to the page in one second, the second is written in the log
/// (ESC-290): an avalanche keeps the page drawing, and the window answering nothing.
pub const LOUD: u32 = 200;
/// A loud second is written once in this long.
const LOUD_EVERY: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct Hub {
    channel: RwLock<Option<Channel<UiEvent>>>,
    meter: Mutex<Meter>,
}

/// The events of the current second, by kind.
#[derive(Default)]
pub struct Meter {
    since: Option<Instant>,
    count: u32,
    kinds: HashMap<Discriminant<UiEvent>, (String, u32)>,
    told: Option<Instant>,
}

impl Meter {
    /// Counts `event`, sent at `now`; the line for the log when the second before was loud.
    pub fn count(&mut self, event: &UiEvent, now: Instant) -> Option<String> {
        let mut line = None;
        let since = *self.since.get_or_insert(now);
        if now.duration_since(since) >= Duration::from_secs(1) {
            let quiet = self
                .told
                .is_some_and(|t| now.duration_since(t) < LOUD_EVERY);
            if self.count > LOUD && !quiet {
                let mut kinds: Vec<&(String, u32)> = self.kinds.values().collect();
                kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                let kinds: Vec<String> = kinds
                    .iter()
                    .take(5)
                    .map(|(name, n)| format!("{name} {n}"))
                    .collect();
                line = Some(format!(
                    "events to the page: {} in one second ({})",
                    self.count,
                    kinds.join(", ")
                ));
                self.told = Some(now);
            }
            self.since = Some(now);
            self.count = 0;
            self.kinds.clear();
        }
        self.count += 1;
        self.kinds
            .entry(std::mem::discriminant(event))
            .or_insert_with(|| (kind_of(event), 0))
            .1 += 1;
        line
    }
}

/// The name of the event's kind (`Git`, `Conv`…), read from the start of its `Debug`, which stops
/// there: never its content.
fn kind_of(event: &UiEvent) -> String {
    struct Name(String);
    impl std::fmt::Write for Name {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            for c in s.chars() {
                if !(c.is_ascii_alphanumeric() || c == '_') {
                    return Err(std::fmt::Error);
                }
                self.0.push(c);
            }
            Ok(())
        }
    }
    let mut name = Name(String::new());
    let _ = std::fmt::Write::write_fmt(&mut name, format_args!("{event:?}"));
    name.0
}

impl Hub {
    pub fn set_channel(&self, channel: Channel<UiEvent>) {
        *self.channel.write() = Some(channel);
    }

    pub fn emit(&self, event: UiEvent) {
        if let Some(line) = self.meter.lock().count(&event, Instant::now()) {
            log::warn!("{line}");
        }
        if let Some(ch) = &*self.channel.read() {
            if let Err(e) = ch.send(event) {
                log::debug!("event channel send failed: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git() -> UiEvent {
        UiEvent::Git {
            project_id: "p1".into(),
            git: Default::default(),
        }
    }

    #[test]
    fn a_loud_second_is_written_with_its_kinds_and_no_content() {
        let mut m = Meter::default();
        let t0 = Instant::now();
        for _ in 0..250 {
            assert_eq!(m.count(&git(), t0), None);
        }
        let line = m.count(&git(), t0 + Duration::from_millis(1001)).unwrap();
        assert_eq!(line, "events to the page: 250 in one second (Git 250)");
        assert!(!line.contains("p1"));
        // Once in 30 s.
        for _ in 0..250 {
            m.count(&git(), t0 + Duration::from_millis(1500));
        }
        assert_eq!(m.count(&git(), t0 + Duration::from_millis(2600)), None);
        for _ in 0..250 {
            m.count(&git(), t0 + Duration::from_secs(32));
        }
        assert!(m.count(&git(), t0 + Duration::from_secs(34)).is_some());
    }

    #[test]
    fn a_quiet_second_is_not_written() {
        let mut m = Meter::default();
        let t0 = Instant::now();
        for _ in 0..LOUD {
            m.count(&git(), t0);
        }
        assert_eq!(m.count(&git(), t0 + Duration::from_secs(2)), None);
    }
}
