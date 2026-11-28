//! The journal (§ 4.9, R8): one JSON object a line on the standard output (journald, the event viewer, the logs of a
//! container), for the company, on its machine.
//!
//! Each connection allowed or refused: the time, the host, the port, the address resolved, the bytes each way, the
//! duration, and the agent and project the server announced (as it said them: the server says them). Never the
//! content; **never the pairing's code nor a private key**: no event carries either, and [`Journal::write`] refuses a
//! line holding a secret the journal was told of ([`Journal::forbid`]), as a last guard.

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

/// Where the lines go: the standard output, or a buffer in the tests.
#[derive(Clone)]
pub struct Journal {
    sink: Arc<Mutex<Box<dyn Write + Send>>>,
    forbidden: Arc<Mutex<Vec<String>>>,
}

impl Journal {
    pub fn stdout() -> Journal {
        Journal::to(Box::new(std::io::stdout()))
    }

    pub fn to(sink: Box<dyn Write + Send>) -> Journal {
        Journal {
            sink: Arc::new(Mutex::new(sink)),
            forbidden: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// A journal into memory, and what it wrote (the tests).
    pub fn memory() -> (Journal, Lines) {
        let lines = Lines::default();
        (Journal::to(Box::new(lines.clone())), lines)
    }

    /// Text that never goes to the journal (a private key in hex, a code): a line holding it is replaced by a line that
    /// says a secret was held back.
    pub fn forbid(&self, secret: impl Into<String>) {
        let secret = secret.into();
        if secret.len() >= 6 {
            self.forbidden.lock().unwrap().push(secret);
        }
    }

    /// One event: `event` and its fields, after `time`.
    pub fn write(&self, event: &str, fields: Value) {
        let mut line = Map::new();
        line.insert("time".into(), Value::String(now_rfc3339()));
        line.insert("event".into(), Value::String(event.into()));
        if let Value::Object(fields) = fields {
            for (k, v) in fields {
                if !v.is_null() {
                    line.insert(k, v);
                }
            }
        }
        let mut text = Value::Object(line).to_string();
        let held = {
            let forbidden = self.forbidden.lock().unwrap();
            forbidden.iter().any(|s| {
                text.contains(s.as_str()) || text.to_lowercase().contains(&s.to_lowercase())
            })
        };
        if held {
            text = serde_json::json!({
                "time": now_rfc3339(),
                "event": "held-back",
                "message": format!("a line of the event « {event} » held a secret and was not written"),
            })
            .to_string();
        }
        let mut sink = self.sink.lock().unwrap();
        let _ = writeln!(sink, "{text}");
        let _ = sink.flush();
    }
}

/// The lines a [`Journal::memory`] wrote.
#[derive(Clone, Default)]
pub struct Lines(Arc<Mutex<Vec<u8>>>);

impl Lines {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }

    /// Each line, read back.
    pub fn events(&self) -> Vec<Value> {
        self.text()
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    /// The lines of one event.
    pub fn of(&self, event: &str) -> Vec<Value> {
        self.events()
            .into_iter()
            .filter(|e| e["event"] == event)
            .collect()
    }
}

impl Write for Lines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `2026-11-27T20:54:00.123Z`, in UTC.
pub fn now_rfc3339() -> String {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    rfc3339(d.as_secs() as i64, d.subsec_millis())
}

fn rfc3339(secs: i64, millis: u32) -> String {
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    // Civil from days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_time_is_rfc3339_in_utc() {
        assert_eq!(rfc3339(0, 0), "1970-01-01T00:00:00.000Z");
        assert_eq!(rfc3339(1_795_812_840, 123), "2026-11-27T20:54:00.123Z");
        assert_eq!(rfc3339(951_782_400, 5), "2000-02-29T00:00:00.005Z");
    }

    #[test]
    fn a_line_is_one_json_object_without_nulls() {
        let (j, lines) = Journal::memory();
        j.write(
            "refused",
            json!({"host": "a.acme.internal", "port": 22, "address": null}),
        );
        let events = lines.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event"], "refused");
        assert_eq!(events[0]["port"], 22);
        assert!(events[0].get("address").is_none());
        assert!(events[0]["time"].as_str().unwrap().ends_with('Z'));
    }

    #[test]
    fn a_secret_never_reaches_it() {
        let (j, lines) = Journal::memory();
        j.forbid("8NUCW6HS");
        j.forbid("abcdef0123456789");
        j.write("x", json!({"message": "the code 8nucw6hs"}));
        j.write("y", json!({"message": "key ABCDEF0123456789"}));
        j.write("z", json!({"message": "fine"}));
        let text = lines.text();
        assert!(!text.to_lowercase().contains("8nucw6hs"));
        assert!(!text.to_lowercase().contains("abcdef0123456789"));
        assert_eq!(lines.of("held-back").len(), 2);
        assert_eq!(lines.of("z").len(), 1);
    }
}
