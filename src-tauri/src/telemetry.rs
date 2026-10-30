//! « Statistiques d'usage »: what the app is used for, sent to Escouade's own Aptabase
//! (aptabase.gagnaire.dev) once the user said yes (`Settings::telemetry`, asked at the first
//! start). An event says which feature was used, with the app's version and the system's; every
//! one carries the install's random id (`DataDir::telemetry_id_file`), so that the users active
//! over 30 days can be counted, and nothing else about the user, their projects or their code.
//!
//! Aptabase's SDK for Tauri is not used: it brings reqwest with its native TLS (OpenSSL on Linux),
//! which every other client of the app would take instead of rustls. This sends what its 1.0.0
//! sends, the same way: events queued, posted 25 at a time every minute and when the app quits.

use crate::paths;
use chrono::{DateTime, SecondsFormat, Utc};
use parking_lot::Mutex;
use serde_json::{json, Map, Value};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// The app's key in Escouade's Aptabase (`SH`: a self-hosted instance).
pub const APP_KEY: &str = "A-SH-9464589961";
pub const ENDPOINT: &str = "https://aptabase.gagnaire.dev/api/v0/events";
/// What the SDK says it is, in its `sdkVersion` (40 characters at most).
const SDK: &str = concat!("escouade@", env!("CARGO_PKG_VERSION"));
/// Aptabase takes 25 events per request at most.
const BATCH: usize = 25;
/// Events that could not leave (no network) wait, up to this many: the oldest go first.
const QUEUE_MAX: usize = 1000;
/// A session ends after this long without an event, as the SDK's.
const SESSION_IDLE: chrono::Duration = chrono::Duration::hours(4);
const FLUSH_EVERY: Duration = Duration::from_secs(60);
/// An app left open for days still counts as used each day.
const ACTIVE_EVERY: Duration = Duration::from_secs(24 * 3600);
/// At quit, the last events have this long to leave.
const LAST_FLUSH: Duration = Duration::from_secs(3);

/// What happened. One name per feature; the props never hold a name, a path or a text.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The app started: its setup, in broad strokes.
    AppStarted {
        language: String,
        accounts: usize,
        projects: usize,
        mcp: bool,
        wsl: bool,
    },
    /// Still open, a day later.
    AppActive,
    ProjectCreated,
    AgentCreated {
        worktree: bool,
    },
    MessageSent,
    TicketCreated,
    TicketStarted {
        autopilot: bool,
    },
    TicketApproved,
    Committed,
    Merged,
    GitSync {
        op: &'static str,
    },
    BranchCreated,
    TerminalOpened,
    LaunchStarted,
    TestRunStarted,
    ClaudeCodeView,
    FileSaved,
    CodeSearched,
    McpEnabled,
    IntegrationConnected {
        service: &'static str,
    },
    TicketsImported {
        service: &'static str,
    },
    AccountCreated,
    /// Where it comes from: "agent" (turned on for one), "project" or "global".
    RemoteControlEnabled {
        from: &'static str,
    },
    /// A dictation gave text: where it went ("composer", "terminal", "ticket") and with which
    /// model; never what was said.
    DictationUsed {
        target: &'static str,
        model: &'static str,
        /// On line: the service's model (ESC-132).
        online_model: Option<String>,
    },
    /// A voice conversation started: the language it is held in, the voice that reads, what makes it (`piper` or the
    /// service on line) and its model; never what is said.
    VoiceConversation {
        lang: &'static str,
        voice: String,
        engine: &'static str,
        model: String,
    },
    /// A plugin installed or updated: whether from the store (STORE-9, docs/STORE.md §5.4); never
    /// which plugin.
    PluginInstalled {
        from_store: bool,
    },
}

impl Event {
    pub fn name(&self) -> &'static str {
        match self {
            Event::AppStarted { .. } => "app_started",
            Event::AppActive => "app_active",
            Event::ProjectCreated => "project_created",
            Event::AgentCreated { .. } => "agent_created",
            Event::MessageSent => "message_sent",
            Event::TicketCreated => "ticket_created",
            Event::TicketStarted { .. } => "ticket_started",
            Event::TicketApproved => "ticket_approved",
            Event::Committed => "committed",
            Event::Merged => "merged",
            Event::GitSync { .. } => "git_sync",
            Event::BranchCreated => "branch_created",
            Event::TerminalOpened => "terminal_opened",
            Event::LaunchStarted => "launch_started",
            Event::TestRunStarted => "test_run_started",
            Event::ClaudeCodeView => "claude_code_view",
            Event::FileSaved => "file_saved",
            Event::CodeSearched => "code_searched",
            Event::McpEnabled => "mcp_enabled",
            Event::IntegrationConnected { .. } => "integration_connected",
            Event::TicketsImported { .. } => "tickets_imported",
            Event::AccountCreated => "account_created",
            Event::RemoteControlEnabled { .. } => "remote_control_enabled",
            Event::DictationUsed { .. } => "dictation_used",
            Event::VoiceConversation { .. } => "voice_conversation",
            Event::PluginInstalled { .. } => "plugin_installed",
        }
    }

    /// Its props, without the install's id.
    fn props(&self) -> Map<String, Value> {
        // Aptabase counts a prop's values: yes and no read better than true and false.
        let yes = |b: bool| json!(if b { "yes" } else { "no" });
        let props = match self {
            Event::AppStarted {
                language,
                accounts,
                projects,
                mcp,
                wsl,
            } => json!({
                "language": language,
                "accounts": accounts,
                "projects": projects,
                "mcp": yes(*mcp),
                "wsl": yes(*wsl),
            }),
            Event::AgentCreated { worktree } => json!({ "worktree": yes(*worktree) }),
            Event::TicketStarted { autopilot } => {
                json!({ "by": if *autopilot { "autopilot" } else { "user" } })
            }
            Event::GitSync { op } => json!({ "op": op }),
            Event::RemoteControlEnabled { from } => json!({ "from": from }),
            Event::DictationUsed {
                target,
                model,
                online_model,
            } => match online_model {
                Some(m) => json!({ "target": target, "model": model, "online_model": m }),
                None => json!({ "target": target, "model": model }),
            },
            Event::VoiceConversation {
                lang,
                voice,
                engine,
                model,
            } => json!({ "lang": lang, "voice": voice, "engine": engine, "model": model }),
            Event::PluginInstalled { from_store } => json!({ "from_store": yes(*from_store) }),
            Event::IntegrationConnected { service } | Event::TicketsImported { service } => {
                json!({ "service": service })
            }
            _ => json!({}),
        };
        match props {
            Value::Object(m) => m,
            _ => Map::new(),
        }
    }
}

/// What every event says of the app and the system it runs on.
#[derive(Debug, Clone)]
pub struct System {
    pub app_version: String,
    pub is_debug: bool,
    pub os_name: String,
    pub os_version: String,
    pub locale: String,
    pub engine_name: String,
    pub engine_version: String,
}

impl System {
    /// This one, as the SDK reads it.
    pub fn current(app_version: String) -> Self {
        let info = os_info::get();
        let os_name = match info.os_type() {
            os_info::Type::Macos => "macOS".to_string(),
            os_info::Type::Windows => "Windows".to_string(),
            other => other.to_string(),
        };
        Self {
            app_version,
            is_debug: cfg!(debug_assertions),
            os_name,
            os_version: info.version().to_string(),
            // Aptabase keeps 10 characters at most (`fr-FR`, `en-US`).
            locale: sys_locale::get_locale()
                .unwrap_or_default()
                .chars()
                .take(10)
                .collect(),
            engine_name: if cfg!(target_os = "windows") {
                "WebView2"
            } else if cfg!(target_os = "macos") {
                "WebKit"
            } else {
                "WebKitGTK"
            }
            .to_string(),
            engine_version: tauri::webview_version().unwrap_or_default(),
        }
    }

    fn props(&self) -> Value {
        json!({
            "isDebug": self.is_debug,
            "osName": self.os_name,
            "osVersion": self.os_version,
            "locale": self.locale,
            "engineName": self.engine_name,
            "engineVersion": self.engine_version,
            "appVersion": self.app_version,
            "sdkVersion": SDK,
        })
    }

    fn user_agent(&self) -> String {
        format!(
            "{}/{} {}/{} {}",
            self.os_name, self.os_version, self.engine_name, self.engine_version, self.locale
        )
    }
}

struct Session {
    id: String,
    touched: DateTime<Utc>,
}

impl Session {
    fn new(now: DateTime<Utc>) -> Self {
        // As the SDK's: the second it started, then 8 random digits.
        let random = u64::from_le_bytes(*uuid::Uuid::new_v4().as_bytes().first_chunk().unwrap());
        let id = now.timestamp() as u64 * 100_000_000 + random % 100_000_000;
        Self {
            id: id.to_string(),
            touched: now,
        }
    }
}

struct Inner {
    /// Set while the user agrees.
    install_id: Option<String>,
    /// Counts the yeses: a batch taken before a no then a yes is not put back under the new id.
    consent: u64,
    session: Session,
    queue: VecDeque<Value>,
}

/// Events out of the queue to be posted, and the consent they were taken under.
struct Batch {
    events: Vec<Value>,
    consent: u64,
}

pub struct Telemetry {
    id_file: PathBuf,
    app_version: String,
    /// Read at the first event (`os_info` may run a program): never while the user said no.
    system: OnceLock<System>,
    /// False in a sandbox (`ESCOUADE_DATA_DIR`: the e2e tests, the demos): nothing is ever sent.
    allowed: bool,
    /// `ENDPOINT`; a local server in the tests.
    endpoint: String,
    inner: Mutex<Inner>,
}

impl Telemetry {
    pub fn new(
        id_file: PathBuf,
        app_version: String,
        consent: Option<bool>,
        allowed: bool,
    ) -> Self {
        let t = Self {
            id_file,
            app_version,
            system: OnceLock::new(),
            allowed,
            endpoint: ENDPOINT.into(),
            inner: Mutex::new(Inner {
                install_id: None,
                consent: 0,
                session: Session::new(Utc::now()),
                queue: VecDeque::new(),
            }),
        };
        t.set_consent(consent);
        t
    }

    /// The user's answer, given or changed. Yes: the install's id is read, or made the first
    /// time. Otherwise the id is forgotten, its file removed, and what waited to leave dropped.
    /// True when the statistics start with it.
    pub fn set_consent(&self, consent: Option<bool>) -> bool {
        if !self.allowed {
            return false;
        }
        let mut inner = self.inner.lock();
        if consent == Some(true) {
            if inner.install_id.is_some() {
                return false;
            }
            inner.install_id = read_or_make_id(&self.id_file);
            inner.consent += 1;
            return inner.install_id.is_some();
        }
        inner.install_id = None;
        inner.queue.clear();
        match std::fs::remove_file(&self.id_file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                log::warn!("telemetry: cannot remove {}: {e}", self.id_file.display())
            }
            _ => {}
        }
        false
    }

    #[cfg(test)]
    pub(crate) fn enabled(&self) -> bool {
        self.inner.lock().install_id.is_some()
    }

    pub fn track(&self, event: Event) {
        self.track_at(event, Utc::now());
    }

    fn track_at(&self, event: Event, now: DateTime<Utc>) {
        if self.inner.lock().install_id.is_none() {
            return;
        }
        // Out of the lock: the first time, it reads the system.
        let system = self.system().props();
        let mut inner = self.inner.lock();
        // Read again: the user may have said no meanwhile.
        let Some(id) = inner.install_id.clone() else {
            return;
        };
        if now - inner.session.touched > SESSION_IDLE {
            inner.session = Session::new(now);
        } else {
            inner.session.touched = now;
        }
        let mut props = event.props();
        props.insert("installId".into(), json!(id));
        let e = json!({
            "timestamp": now.to_rfc3339_opts(SecondsFormat::Millis, true),
            "sessionId": inner.session.id,
            "eventName": event.name(),
            "systemProps": system,
            "props": props,
        });
        if inner.queue.len() >= QUEUE_MAX {
            inner.queue.pop_front();
        }
        inner.queue.push_back(e);
    }

    /// The next events to post, 25 at most, out of the queue.
    fn take_batch(&self) -> Batch {
        let mut inner = self.inner.lock();
        let n = inner.queue.len().min(BATCH);
        Batch {
            events: inner.queue.drain(..n).collect(),
            consent: inner.consent,
        }
    }

    /// Events that could not leave, back at the front of the queue, unless the user said no
    /// since they were taken.
    fn put_back(&self, batch: Batch) {
        let mut inner = self.inner.lock();
        if inner.install_id.is_none() || inner.consent != batch.consent {
            return;
        }
        for e in batch.events.into_iter().rev() {
            inner.queue.push_front(e);
        }
        inner.queue.truncate(QUEUE_MAX);
    }

    fn is_empty(&self) -> bool {
        self.inner.lock().queue.is_empty()
    }

    /// Everything queued, posted; what fails waits for the next time.
    pub async fn flush(&self, http: &reqwest::Client) {
        loop {
            let batch = self.take_batch();
            if batch.events.is_empty() {
                return;
            }
            let sent = http
                .post(&self.endpoint)
                .header("App-Key", APP_KEY)
                .json(&batch.events)
                .send()
                .await;
            let status = sent.as_ref().map(|r| r.status()).ok();
            match status {
                Some(s) if s.is_success() => {}
                // Refused (an event Aptabase finds invalid): sending it again changes nothing.
                // Too many requests, or too slow, is for later.
                Some(s)
                    if s.is_client_error()
                        && s != reqwest::StatusCode::TOO_MANY_REQUESTS
                        && s != reqwest::StatusCode::REQUEST_TIMEOUT =>
                {
                    log::warn!("telemetry: {} events refused ({s})", batch.events.len());
                }
                _ => {
                    self.put_back(batch);
                    return;
                }
            }
        }
    }

    fn system(&self) -> &System {
        self.system
            .get_or_init(|| System::current(self.app_version.clone()))
    }

    /// What is waiting to leave.
    #[cfg(test)]
    pub(crate) fn pending(&self) -> Vec<Value> {
        self.inner.lock().queue.iter().cloned().collect()
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(self.system().user_agent())
            .build()
            .unwrap_or_default()
    }

    /// The events leave every minute; a day of the app open counts as a day used.
    pub fn spawn(self: &Arc<Self>) {
        let t = self.clone();
        tauri::async_runtime::spawn(async move {
            // Made at the first event to send: never while the user said no.
            let mut http = None;
            let mut active = tokio::time::Instant::now();
            loop {
                tokio::time::sleep(FLUSH_EVERY).await;
                if active.elapsed() >= ACTIVE_EVERY {
                    active = tokio::time::Instant::now();
                    t.track(Event::AppActive);
                }
                if !t.is_empty() {
                    t.flush(http.get_or_insert_with(|| t.client())).await;
                }
            }
        });
    }

    /// At quit: what is left leaves, if it can within 3 seconds. On a thread of its own, so as
    /// to run from anywhere, the app's runtime included.
    pub fn flush_at_exit(&self) {
        if self.is_empty() {
            return;
        }
        let http = self.client();
        std::thread::scope(|s| {
            s.spawn(|| {
                tauri::async_runtime::block_on(async {
                    let _ = tokio::time::timeout(LAST_FLUSH, self.flush(&http)).await;
                })
            });
        });
    }
}

/// A ticket system, as the statistics name it.
pub fn service(s: crate::model::Service) -> &'static str {
    use crate::model::Service;
    match s {
        Service::Jira => "jira",
        Service::Trello => "trello",
        Service::Github => "github",
        Service::Linear => "linear",
        Service::Gitlab => "gitlab",
        Service::Clickup => "clickup",
        Service::Azure => "azure",
    }
}

fn read_or_make_id(file: &std::path::Path) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(file) {
        if let Ok(id) = uuid::Uuid::parse_str(text.trim()) {
            return Some(id.to_string());
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    match paths::write_atomic(file, id.as_bytes()) {
        Ok(()) => Some(id),
        Err(e) => {
            // Without its id, an install would count as a new one at each start: nothing is sent.
            log::warn!("telemetry: cannot write {}: {e}", file.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system() -> System {
        System {
            app_version: "1.8.0".into(),
            is_debug: false,
            os_name: "Windows".into(),
            os_version: "10.0.26200".into(),
            locale: "fr-FR".into(),
            engine_name: "WebView2".into(),
            engine_version: "141.0".into(),
        }
    }

    impl Telemetry {
        fn with(id_file: PathBuf, system: System, consent: Option<bool>, allowed: bool) -> Self {
            let t = Telemetry::new(id_file, system.app_version.clone(), consent, allowed);
            let _ = t.system.set(system);
            t
        }
    }

    fn dir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("escouade-telemetry-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn nothing_is_kept_until_the_user_says_yes() {
        let d = dir("no-answer");
        for consent in [None, Some(false)] {
            let t = Telemetry::with(d.join("telemetry-id"), system(), consent, true);
            t.track(Event::MessageSent);
            assert!(!t.enabled());
            assert!(t.take_batch().events.is_empty());
            assert!(!d.join("telemetry-id").exists(), "no id without a yes");
        }
    }

    #[test]
    fn an_event_says_what_aptabase_reads_with_the_installs_id() {
        let d = dir("event");
        let started = Utc::now().timestamp() as u64;
        let t = Telemetry::with(d.join("telemetry-id"), system(), Some(true), true);
        t.track_at(
            Event::AgentCreated { worktree: true },
            at("2026-10-13T20:00:00Z"),
        );
        let batch = t.take_batch().events;
        assert_eq!(batch.len(), 1);
        let e = &batch[0];
        assert_eq!(e["eventName"], "agent_created");
        assert_eq!(e["timestamp"], "2026-10-13T20:00:00.000Z");
        // The second the session started (the app did), then 8 random digits.
        let session: u64 = e["sessionId"].as_str().unwrap().parse().unwrap();
        assert!((started..started + 5).contains(&(session / 100_000_000)));
        assert_eq!(
            e["systemProps"],
            json!({
                "isDebug": false,
                "osName": "Windows",
                "osVersion": "10.0.26200",
                "locale": "fr-FR",
                "engineName": "WebView2",
                "engineVersion": "141.0",
                "appVersion": "1.8.0",
                "sdkVersion": SDK,
            })
        );
        let id = std::fs::read_to_string(d.join("telemetry-id")).unwrap();
        assert_eq!(
            uuid::Uuid::parse_str(id.trim()).unwrap().get_version_num(),
            4
        );
        assert_eq!(
            e["props"],
            json!({ "installId": id.trim(), "worktree": "yes" })
        );
        assert!(SDK.len() <= 40);
    }

    #[test]
    fn the_install_keeps_its_id_from_one_start_to_the_next() {
        let d = dir("same-id");
        let id = |t: &Telemetry| {
            t.track(Event::MessageSent);
            t.take_batch().events[0]["props"]["installId"].clone()
        };
        let first = id(&Telemetry::with(
            d.join("telemetry-id"),
            system(),
            Some(true),
            true,
        ));
        let again = id(&Telemetry::with(
            d.join("telemetry-id"),
            system(),
            Some(true),
            true,
        ));
        assert_eq!(first, again);
    }

    #[test]
    fn saying_no_forgets_the_id_and_drops_what_waited() {
        let d = dir("opt-out");
        let file = d.join("telemetry-id");
        let t = Telemetry::with(file.clone(), system(), Some(true), true);
        t.track(Event::MessageSent);
        let before = std::fs::read_to_string(&file).unwrap();
        t.set_consent(Some(false));
        assert!(!t.enabled());
        assert!(!file.exists());
        assert!(t.take_batch().events.is_empty());
        t.track(Event::MessageSent);
        assert!(t.take_batch().events.is_empty());
        // Yes again: a new install, as far as the statistics can tell.
        t.set_consent(Some(true));
        t.track(Event::MessageSent);
        let after = t.take_batch().events[0]["props"]["installId"].clone();
        assert_ne!(after, json!(before.trim()));
    }

    #[test]
    fn a_sandbox_never_sends_whatever_its_settings_say() {
        let d = dir("sandbox");
        let t = Telemetry::with(d.join("telemetry-id"), system(), Some(true), false);
        t.set_consent(Some(true));
        t.track(Event::MessageSent);
        assert!(!t.enabled());
        assert!(t.take_batch().events.is_empty());
        assert!(!d.join("telemetry-id").exists());
    }

    #[test]
    fn a_session_lasts_until_four_hours_without_an_event() {
        let d = dir("session");
        let t = Telemetry::with(d.join("telemetry-id"), system(), Some(true), true);
        t.track_at(Event::MessageSent, at("2026-10-13T08:00:00Z"));
        t.track_at(Event::MessageSent, at("2026-10-13T11:59:00Z"));
        t.track_at(Event::MessageSent, at("2026-10-13T15:59:00Z"));
        t.track_at(Event::MessageSent, at("2026-10-13T20:00:00Z"));
        let s: Vec<_> = t
            .take_batch()
            .events
            .iter()
            .map(|e| e["sessionId"].clone())
            .collect();
        assert_eq!(s[0], s[1]);
        assert_eq!(s[1], s[2]);
        assert_ne!(s[2], s[3]);
    }

    #[test]
    fn events_leave_25_at_a_time_and_wait_when_they_cannot() {
        let d = dir("batch");
        let t = Telemetry::with(d.join("telemetry-id"), system(), Some(true), true);
        for _ in 0..30 {
            t.track(Event::MessageSent);
        }
        let first = t.take_batch();
        assert_eq!(first.events.len(), 25);
        t.put_back(first);
        assert_eq!(t.take_batch().events.len(), 25);
        assert_eq!(t.take_batch().events.len(), 5);
        assert!(t.take_batch().events.is_empty());
    }

    #[test]
    fn without_network_the_oldest_events_go_first() {
        let d = dir("cap");
        let t = Telemetry::with(d.join("telemetry-id"), system(), Some(true), true);
        t.track(Event::TicketCreated);
        for _ in 0..QUEUE_MAX {
            t.track(Event::MessageSent);
        }
        let mut all = Vec::new();
        loop {
            let b = t.take_batch().events;
            if b.is_empty() {
                break;
            }
            all.extend(b);
        }
        assert_eq!(all.len(), QUEUE_MAX);
        assert!(all.iter().all(|e| e["eventName"] == "message_sent"));
    }

    #[test]
    fn each_feature_has_a_name_of_its_own_and_props_without_anything_personal() {
        let events = [
            Event::AppStarted {
                language: "fr".into(),
                accounts: 2,
                projects: 3,
                mcp: true,
                wsl: false,
            },
            Event::AppActive,
            Event::ProjectCreated,
            Event::AgentCreated { worktree: false },
            Event::MessageSent,
            Event::TicketCreated,
            Event::TicketStarted { autopilot: true },
            Event::TicketApproved,
            Event::Committed,
            Event::Merged,
            Event::GitSync { op: "push" },
            Event::BranchCreated,
            Event::TerminalOpened,
            Event::LaunchStarted,
            Event::TestRunStarted,
            Event::ClaudeCodeView,
            Event::FileSaved,
            Event::CodeSearched,
            Event::McpEnabled,
            Event::IntegrationConnected { service: "linear" },
            Event::TicketsImported { service: "jira" },
            Event::AccountCreated,
            Event::RemoteControlEnabled { from: "agent" },
        ];
        let names: std::collections::HashSet<_> = events.iter().map(Event::name).collect();
        assert_eq!(names.len(), events.len());
        for n in &names {
            assert!(
                n.len() <= 60 && n.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{n}"
            );
        }
        assert_eq!(
            Value::Object(events[0].props()),
            json!({ "language": "fr", "accounts": 2, "projects": 3, "mcp": "yes", "wsl": "no" })
        );
        assert_eq!(
            Value::Object(Event::TicketStarted { autopilot: false }.props()),
            json!({ "by": "user" })
        );
        assert_eq!(
            Value::Object(Event::GitSync { op: "pull" }.props()),
            json!({ "op": "pull" })
        );
        assert_eq!(
            Value::Object(Event::TicketsImported { service: "jira" }.props()),
            json!({ "service": "jira" })
        );
        for from in ["agent", "project", "global"] {
            assert_eq!(
                Value::Object(Event::RemoteControlEnabled { from }.props()),
                json!({ "from": from })
            );
        }
        assert!(Event::MessageSent.props().is_empty());
    }

    #[test]
    fn a_batch_taken_before_a_no_then_a_yes_is_not_put_back_under_the_new_id() {
        let d = dir("no-then-yes");
        let t = Telemetry::with(d.join("telemetry-id"), system(), Some(true), true);
        t.track(Event::MessageSent);
        let in_flight = t.take_batch();
        t.set_consent(Some(false));
        t.set_consent(Some(true));
        t.put_back(in_flight);
        assert!(t.pending().is_empty());
        // Said no while it was on its way: dropped too.
        t.track(Event::MessageSent);
        let in_flight = t.take_batch();
        t.set_consent(Some(false));
        t.put_back(in_flight);
        assert!(t.pending().is_empty());
    }

    /// A server answering each request with the next status of `statuses` (200 once they run
    /// out), and what it was sent: each request's App-Key header and body.
    async fn server(statuses: Vec<u16>) -> (String, Arc<Mutex<Vec<(String, Value)>>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/api/v0/events", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        tokio::spawn(async move {
            let mut statuses = statuses.into_iter();
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    return;
                };
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let (head, body) = loop {
                    let n = sock.read(&mut chunk).await.unwrap();
                    buf.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&buf).to_string();
                    let Some(end) = text.find("\r\n\r\n") else {
                        continue;
                    };
                    let head = text[..end].to_lowercase();
                    let len: usize = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .map_or(0, |v| v.trim().parse().unwrap());
                    if buf.len() >= end + 4 + len {
                        break (head, buf[end + 4..end + 4 + len].to_vec());
                    }
                };
                let key = head
                    .lines()
                    .find_map(|l| l.strip_prefix("app-key:"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                log.lock()
                    .push((key, serde_json::from_slice(&body).unwrap()));
                let status = statuses.next().unwrap_or(200);
                let reply = format!(
                    "HTTP/1.1 {status} X\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                );
                let _ = sock.write_all(reply.as_bytes()).await;
            }
        });
        (url, seen)
    }

    fn sending_to(url: String, name: &str) -> Telemetry {
        let d = dir(name);
        let mut t = Telemetry::with(d.join("telemetry-id"), system(), Some(true), true);
        t.endpoint = url;
        t
    }

    #[tokio::test]
    async fn the_events_leave_25_at_a_time_with_the_apps_key() {
        let (url, seen) = server(vec![]).await;
        let t = sending_to(url, "post");
        for _ in 0..30 {
            t.track(Event::MessageSent);
        }
        t.flush(&reqwest::Client::new()).await;
        assert!(t.pending().is_empty());
        let seen = seen.lock();
        let sizes: Vec<_> = seen
            .iter()
            .map(|(_, b)| b.as_array().unwrap().len())
            .collect();
        assert_eq!(sizes, [25, 5]);
        assert!(seen.iter().all(|(key, _)| key == &APP_KEY.to_lowercase()));
        assert_eq!(seen[0].1[0]["eventName"], "message_sent");
    }

    #[tokio::test]
    async fn events_the_server_could_not_take_wait_for_the_next_time() {
        for status in [500, 503, 429, 408] {
            let (url, seen) = server(vec![status]).await;
            let t = sending_to(url, &format!("retry-{status}"));
            t.track(Event::MessageSent);
            t.flush(&reqwest::Client::new()).await;
            assert_eq!(t.pending().len(), 1, "{status}");
            t.flush(&reqwest::Client::new()).await;
            assert!(t.pending().is_empty(), "{status}");
            assert_eq!(seen.lock().len(), 2);
        }
    }

    #[tokio::test]
    async fn events_the_server_refuses_are_not_sent_again() {
        let (url, seen) = server(vec![400]).await;
        let t = sending_to(url, "refused");
        t.track(Event::MessageSent);
        t.flush(&reqwest::Client::new()).await;
        assert!(t.pending().is_empty());
        assert_eq!(seen.lock().len(), 1);
    }

    #[tokio::test]
    async fn without_network_the_events_wait() {
        // Nothing listens there any more.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/api/v0/events", listener.local_addr().unwrap());
        drop(listener);
        let t = sending_to(url, "offline");
        t.track(Event::MessageSent);
        t.flush(&reqwest::Client::new()).await;
        assert_eq!(t.pending().len(), 1);
    }
}
