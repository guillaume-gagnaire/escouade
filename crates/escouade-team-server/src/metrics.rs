//! Prometheus metrics, in its text format: counts and sizes, by route template and status, never an id of a team, a
//! project or a device (`docs/TEAM-SERVER.md` §14). Served on their own address (`--metrics-listen`), not on the
//! exposed one.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;

use crate::db::Counts;

#[derive(Default)]
pub struct Metrics {
    /// (method, route template, status) → requests.
    requests: Mutex<BTreeMap<(String, String, u16), u64>>,
    pub auth_failures: AtomicU64,
    pub rate_limited: AtomicU64,
    pub team_entries: AtomicU64,
    pub project_entries: AtomicU64,
    pub entry_bytes: AtomicU64,
    pub epoch_conflicts: AtomicU64,
    /// Snapshots a device found unreadable (SR2-01).
    pub unreadable_snapshots: AtomicU64,
    /// Tickets taken or given back (TS-6).
    pub claims: AtomicU64,
    /// Claims refused: another device held the ticket.
    pub claims_refused: AtomicU64,
    /// Calls refused: the device had no access to the project.
    pub refused_access: AtomicU64,
    pub files_written: AtomicU64,
    pub file_bytes: AtomicU64,
    pub live_connections: AtomicI64,
    pub live_frames: AtomicU64,
}

fn escape(label: &str) -> String {
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

impl Metrics {
    pub fn request(&self, method: &str, route: &str, status: u16) {
        let mut map = self.requests.lock().unwrap_or_else(|e| e.into_inner());
        *map.entry((method.to_string(), route.to_string(), status))
            .or_default() += 1;
    }

    pub fn inc(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn render(&self, counts: Option<Counts>) -> String {
        let mut out = String::new();
        out.push_str("# HELP escouade_team_http_requests_total Requests, by method, route template and status.\n");
        out.push_str("# TYPE escouade_team_http_requests_total counter\n");
        for ((method, route, status), n) in self
            .requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
        {
            let _ = writeln!(
                out,
                "escouade_team_http_requests_total{{method=\"{}\",route=\"{}\",status=\"{status}\"}} {n}",
                escape(method),
                escape(route)
            );
        }
        let counters: [(&str, &str, &AtomicU64); 13] = [
            (
                "escouade_team_auth_failures_total",
                "Refused sign-ins and sessions.",
                &self.auth_failures,
            ),
            (
                "escouade_team_rate_limited_total",
                "Requests refused by a rate limit.",
                &self.rate_limited,
            ),
            (
                "escouade_team_team_entries_total",
                "Entries appended to the teams' streams.",
                &self.team_entries,
            ),
            (
                "escouade_team_project_entries_total",
                "Entries appended to the projects' streams.",
                &self.project_entries,
            ),
            (
                "escouade_team_entry_bytes_total",
                "Bytes of the entries appended.",
                &self.entry_bytes,
            ),
            (
                "escouade_team_epoch_conflicts_total",
                "Commits refused: their epoch had one already.",
                &self.epoch_conflicts,
            ),
            (
                "escouade_team_unreadable_snapshots_total",
                "Snapshots a device found unreadable and flagged.",
                &self.unreadable_snapshots,
            ),
            (
                "escouade_team_claims_total",
                "Tickets taken or given back.",
                &self.claims,
            ),
            (
                "escouade_team_claims_refused_total",
                "Claims refused: another device held the ticket.",
                &self.claims_refused,
            ),
            (
                "escouade_team_access_refused_total",
                "Calls refused: no access to the project.",
                &self.refused_access,
            ),
            (
                "escouade_team_files_written_total",
                "Encrypted files written.",
                &self.files_written,
            ),
            (
                "escouade_team_file_bytes_total",
                "Bytes of the encrypted files written.",
                &self.file_bytes,
            ),
            (
                "escouade_team_live_frames_total",
                "Frames sent to live connections.",
                &self.live_frames,
            ),
        ];
        for (name, help, value) in counters {
            let _ = writeln!(
                out,
                "# HELP {name} {help}\n# TYPE {name} counter\n{name} {}",
                value.load(Ordering::Relaxed)
            );
        }
        let _ = writeln!(
            out,
            "# HELP escouade_team_live_connections Open live connections.\n# TYPE escouade_team_live_connections gauge\nescouade_team_live_connections {}",
            self.live_connections.load(Ordering::Relaxed)
        );
        if let Some(c) = counts {
            for (name, help, value) in [
                ("escouade_team_teams", "Teams on this server.", c.teams),
                (
                    "escouade_team_devices",
                    "Devices on this server, revoked ones included.",
                    c.devices,
                ),
                (
                    "escouade_team_revoked_devices",
                    "Revoked devices on this server.",
                    c.revoked_devices,
                ),
            ] {
                let _ = writeln!(
                    out,
                    "# HELP {name} {help}\n# TYPE {name} gauge\n{name} {value}"
                );
            }
        }
        out
    }
}
