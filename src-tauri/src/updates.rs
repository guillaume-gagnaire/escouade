//! The app's own updates (GitHub Releases), checked, downloaded and installed here rather than by
//! the window through the updater plugin: they then follow the network settings as they are now
//! (the proxy, TLS verification off), which the plugin only takes from its build-time
//! configuration.
//!
//! A version found is downloaded at once, its signature checked, and kept until it installs:
//! when the user restarts the app for it, when the app restarts by itself once at rest (`busy`,
//! `Updates::advance`), or when the app is closed, which then stays closed. The app always stops
//! cleanly first, its sessions and its state saved (`install_ready`).

use crate::core::Core;
use crate::i18n::{self, Lang};
use crate::model::{now_ms, AgentStatus, Settings, UiEvent};
use crate::paths::DataDir;
use anyhow::{anyhow, bail, Result};
use parking_lot::Mutex;
use reqwest::header::USER_AGENT;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

mod net;
use net::{probe, Failure, Network, Route, Via, What, RELEASES_URL};
#[cfg(test)]
mod fake_server;
#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod net_tests;

/// How long a check may take.
const TIMEOUT: Duration = Duration::from_secs(20);

/// How long a check waits before trying again after a failure that may pass (a release whose files
/// are still being added, a link renewed, a network that comes back).
const RETRY_PAUSE: Duration = Duration::from_secs(3);

/// How long a download may take: a large installer behind a slow proxy takes its time, but one
/// that hangs is tried again at a later check.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// How long the app's clean stop before an install may take: past it, the update installs all
/// the same (a stop that hangs would otherwise keep the app from ever updating).
pub const STOP_LIMIT: Duration = Duration::from_secs(15);

/// How long before an automatic restart the user is warned, in ms.
pub const WARNING: i64 = 30_000;

/// Without a key or mouse input in the window for this long (ms), the user is away.
pub const AWAY: i64 = 5 * 60_000;

/// « Plus tard » holds off the automatic restart for this long (ms).
pub const POSTPONE: i64 = 30 * 60_000;

/// How often the app looks whether it may restart by itself.
const TICK: Duration = Duration::from_secs(2);

/// An update found, as the window is told of it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// What downloads or frees it.
    pub id: u32,
    pub version: String,
    pub notes: String,
}

/// What was found and not downloaded or freed yet, by id: each check holds the proxy and the
/// signature it was made with, for its download.
pub struct Kept<T> {
    next: AtomicU32,
    items: Mutex<HashMap<u32, T>>,
}

impl<T> Default for Kept<T> {
    fn default() -> Self {
        Self {
            next: AtomicU32::new(1),
            items: Mutex::default(),
        }
    }
}

impl<T> Kept<T> {
    pub fn keep(&self, item: T) -> u32 {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        self.items.lock().insert(id, item);
        id
    }

    pub fn take(&self, id: u32) -> Option<T> {
        self.items.lock().remove(&id)
    }

    pub fn free(&self, id: u32) {
        self.items.lock().remove(&id);
    }
}

/// What a check or a download fails with, as the window is told of it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trouble {
    /// The cause: the window puts what was being done before it.
    pub message: String,
    /// To be shown now: the user asked for it, or the failures have gone on long enough (`Streak`).
    pub show: bool,
    /// Where to download the update in a browser, for a download that failed.
    pub page: Option<String>,
}

/// What a check found, as the window is told of it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Checked {
    Found { update: Found },
    None,
    Failed { trouble: Trouble },
}

/// How a download ended, as the window is told of it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Downloaded {
    /// The update is downloaded, its signature checked: it installs.
    Ready,
    /// A newer one started downloading meanwhile, or this one was withdrawn.
    Superseded,
    Failed {
        trouble: Trouble,
    },
}

/// Failures that follow one another, for an error that is shown only when it goes on: a check
/// every few minutes that fails because the network is down for a moment is no one's business.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Streak {
    /// The first of the failures, in ms.
    since: i64,
    /// The last of them, in ms.
    last: i64,
    count: u32,
    /// The series has been shown by the rule, once.
    told: bool,
}

/// How long the failures must have gone on to be shown (ms), and how many they must be.
pub const SHOW_AFTER: i64 = 60 * 60_000;
pub const SHOW_AFTER_FAILURES: u32 = 3;

/// A series whose last failure is older than this (ms) is over: the checks come every few
/// minutes, so a longer silence is a sleep or a network that was away, and a failure before it
/// and two after it are no series.
pub const STREAK_GAP: i64 = 30 * 60_000;

impl Streak {
    /// Another failure at `now` (ms). True when it is to be shown: `manual` (the user asked and
    /// waits for an answer), or the failures have gone on for `SHOW_AFTER`, `SHOW_AFTER_FAILURES`
    /// of them, and the series has not been shown yet. A series that went quiet for more than
    /// `STREAK_GAP` starts over with this one.
    fn failed(&mut self, manual: bool, now: i64) -> bool {
        if self.count > 0 && now - self.last > STREAK_GAP {
            *self = Self::default();
        }
        if self.count == 0 {
            self.since = now;
        }
        self.last = now;
        self.count = self.count.saturating_add(1);
        let due = !self.told && self.count >= SHOW_AFTER_FAILURES && now - self.since >= SHOW_AFTER;
        // What the rule showed is not shown again; what the user asked for does not spend the rule.
        self.told |= due;
        manual || due
    }

    /// A success ends the series.
    fn succeeded(&mut self) {
        *self = Self::default();
    }
}

type Fetching<'a> = Pin<Box<dyn Future<Output = Result<Box<dyn Package>>> + Send + 'a>>;

type Reading<'a> = Pin<Box<dyn Future<Output = Result<Option<Arc<dyn Release>>>> + Send + 'a>>;

/// An update a check found: the plugin's, or a test's.
pub trait Release: Send + Sync {
    fn version(&self) -> &str;
    fn notes(&self) -> &str;
    /// Downloads it and checks its signature.
    fn fetch(&self) -> Fetching<'_>;
    /// Reads `latest.json` again, a new check with the same settings: what it finds now. A link
    /// that expired, a release published again: the download is tried once more with this.
    fn again(&self) -> Reading<'_>;
}

/// What runs right before the app goes for an install (`Package::install`).
pub type Hook = Arc<dyn Fn() + Send + Sync>;

/// A downloaded update, its signature checked: the plugin's, or a test's.
pub trait Package: Send + Sync {
    /// Puts the new version in place, `before_exit` run before the app goes: on Windows right
    /// before the installer starts (the plugin's `on_before_exit`), which then ends the app, so
    /// that this only returns when it fails (with `relaunch`, the installer starts the new
    /// version); on macOS once the new bundle replaced the app's.
    fn install(&self, relaunch: bool, before_exit: Hook) -> Result<()>;
}

/// The plugin's `on_before_exit`, set when its update is checked: what the install under way
/// hands it.
type Slot = Arc<Mutex<Option<Hook>>>;

/// The update the plugin found, with the way it was found: a download goes the same way, and
/// latest.json is read again the same way (`Release::again`).
struct PluginRelease<R: Runtime> {
    update: Update,
    slot: Slot,
    /// The route the check went through: the one that worked.
    route: Route,
    checker: Arc<Checker<R>>,
}

impl<R: Runtime> PluginRelease<R> {
    fn package(&self, update: Update, bytes: Vec<u8>) -> Box<dyn Package> {
        Box::new(PluginPackage {
            update,
            bytes,
            slot: self.slot.clone(),
        })
    }

    /// The download failed: what answered is read (`net::probe`), and a proxy that is not there is
    /// gone around once, the way the check does. One that answered is never: what it refuses is
    /// its company's rule.
    async fn refused(
        &self,
        mut update: Update,
        error: tauri_plugin_updater::Error,
    ) -> Result<Box<dyn Package>> {
        let net = &self.checker.net;
        let accept = "application/octet-stream";
        let mut failure = probe(
            &update.download_url,
            net,
            self.route,
            accept,
            What::Download,
            net::chain(&error),
        )
        .await;
        // The link comes from latest.json: the browser is sent to its release only on the origins
        // that file is known to be served from (`net::release_page`).
        failure.trusted = self.checker.endpoints.clone();
        if matches!(net.via(self.route), Via::Proxy(_)) && failure.proxy_unreachable() {
            update.proxy = None;
            update.no_proxy = true;
            match update.download(|_, _| {}, || {}).await {
                Ok(bytes) => {
                    log::warn!("download through the proxy failed ({failure}), direct worked");
                    return Ok(self.package(update, bytes));
                }
                Err(e) => {
                    let direct = probe(
                        &update.download_url,
                        net,
                        Route::Direct,
                        accept,
                        What::Download,
                        net::chain(&e),
                    )
                    .await;
                    failure.direct = Some(Box::new(direct));
                }
            }
        }
        Err(anyhow::Error::new(failure))
    }
}

impl<R: Runtime> Release for PluginRelease<R> {
    fn version(&self) -> &str {
        &self.update.version
    }

    fn notes(&self) -> &str {
        self.update.body.as_deref().unwrap_or_default()
    }

    fn fetch(&self) -> Fetching<'_> {
        Box::pin(async move {
            // The check's limit would cut the download (it bounds the whole request).
            let mut update = self.update.clone();
            update.timeout = Some(DOWNLOAD_TIMEOUT);
            match update.download(|_, _| {}, || {}).await {
                Ok(bytes) => Ok(self.package(update, bytes)),
                Err(e) => self.refused(update, e).await,
            }
        })
    }

    fn again(&self) -> Reading<'_> {
        Box::pin(async move {
            let found = self.checker.find(self.route).await?;
            Ok(found.map(|c| Arc::new(self.checker.release(c)) as Arc<dyn Release>))
        })
    }
}

struct PluginPackage {
    update: Update,
    bytes: Vec<u8>,
    slot: Slot,
}

impl Package for PluginPackage {
    fn install(&self, relaunch: bool, before_exit: Hook) -> Result<()> {
        #[cfg(windows)]
        {
            *self.slot.lock() = Some(before_exit);
            self.update
                .clone()
                .restart_after_install(relaunch)
                .install(&self.bytes)?;
        }
        #[cfg(not(windows))]
        {
            // The running app is not touched by the bundle replaced under it: it stops after.
            let _ = (relaunch, &self.slot);
            self.update.install(&self.bytes)?;
            before_exit();
        }
        Ok(())
    }
}

/// The update downloaded, ready to install.
pub struct Ready {
    pub version: String,
    pub notes: String,
    package: Box<dyn Package>,
}

/// What the window says of itself, for an automatic restart.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Presence {
    /// A modal is open.
    pub modal: bool,
    /// A test launch is being prepared or started (« ▶ Tester »).
    pub testing: bool,
    /// When the window last had a key or mouse input (ms).
    pub active_at: i64,
}

/// The automatic restart planned.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Plan {
    /// When it comes, the user warned: ms.
    at: Option<i64>,
    /// None before (« Plus tard »): ms.
    not_before: i64,
}

/// What the automatic restart does at a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Wait,
    /// Planned for then (ms): the user is warned.
    Warn(i64),
    /// The one planned is called off.
    Cancel,
    Restart,
}

/// What the app is doing at one moment, as far as restarting it goes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    /// Agents in a turn.
    pub turns: usize,
    /// Agents asking a question, or for a permission.
    pub questions: usize,
    /// Agents whose new worktree is being set up.
    pub setups: usize,
    /// Tickets being validated (their merge, pull request or push).
    pub validations: usize,
    /// Work under way outside of a turn (`Core::working`): a ticket being started, a validation
    /// being cleaned up, a message being delivered, a merge asked for by hand.
    pub works: usize,
    /// Subagents, workflows, commands and monitors running in the background of the agents that
    /// have a process (not archived ones): rows, not agents.
    pub background: usize,
    /// Launch commands and steps of test launches still running.
    pub launches: usize,
    /// Since an integrated terminal's last input or output, in ms (`i64::MAX` without one).
    pub terminal_quiet: i64,
    /// A test launch is being prepared or started.
    pub testing: bool,
    /// Files the editor holds unsaved.
    pub unsaved: usize,
    /// A modal is open in the window.
    pub modal: bool,
    /// Since the window's last key or mouse input, in ms.
    pub inactive: i64,
    /// The window is on screen: shown, not minimized.
    pub visible: bool,
}

/// What keeps the app from restarting by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Busy {
    Turn,
    Question,
    Setup,
    Validation,
    Work,
    Background,
    Launch,
    Terminal,
    Testing,
    Unsaved,
    Modal,
    Activity,
}

/// Why the app is not at rest, if it is not: a restart now would cut someone's work, lose a
/// file or surprise the user at the keyboard.
pub fn busy(s: &Snapshot) -> Option<Busy> {
    let why = [
        (s.turns > 0, Busy::Turn),
        (s.questions > 0, Busy::Question),
        (s.setups > 0, Busy::Setup),
        (s.validations > 0, Busy::Validation),
        (s.works > 0, Busy::Work),
        // A command, a monitor, a subagent or a workflow running in the background lives in the
        // agent's process: a restart would kill it (a long build, a watcher), turn over or not.
        (s.background > 0, Busy::Background),
        // A dev server, a watcher, a test launch's step: running, whatever they print.
        (s.launches > 0, Busy::Launch),
        // An idle terminal does not count: one used lately may be running something.
        (s.terminal_quiet < AWAY, Busy::Terminal),
        (s.testing, Busy::Testing),
        (s.unsaved > 0, Busy::Unsaved),
        (s.modal, Busy::Modal),
        // A window out of sight has no one at it, however recently it was used.
        (s.visible && s.inactive < AWAY, Busy::Activity),
    ];
    why.into_iter().find_map(|(on, why)| on.then_some(why))
}

/// The automatic restart's next step (`on`: the setting is on and an update is ready).
fn step(plan: &Plan, on: bool, busy: Option<Busy>, now: i64) -> Step {
    match plan.at {
        None if on && busy.is_none() && now >= plan.not_before => Step::Warn(now + WARNING),
        None => Step::Wait,
        Some(_) if !on || busy.is_some() => Step::Cancel,
        Some(at) if now >= at => Step::Restart,
        Some(_) => Step::Wait,
    }
}

/// The checks found, the update downloaded, and its automatic restart.
pub struct Updates {
    found: Kept<Arc<dyn Release>>,
    ready: Mutex<Option<Arc<Ready>>>,
    /// Bumped by each download started and by a withdrawal: a download that ends after is not
    /// kept.
    downloads: AtomicU64,
    /// An install under way: none other starts.
    installing: AtomicBool,
    /// The install under way has had the app leave (`leave`): the app's end waits for it no more
    /// (`wait_install`).
    left: AtomicBool,
    presence: Mutex<Presence>,
    plan: Mutex<Plan>,
    /// The update installed since the last start, for the window (told once).
    installed: Mutex<Option<Installed>>,
    /// A version that did not install at the last try: no automatic restart for it.
    failed: Mutex<Option<String>>,
    /// The checks that failed one after the other, and the downloads: two series, for a download
    /// refused every time must be told even though the checks go through.
    check_streak: Mutex<Streak>,
    download_streak: Mutex<Streak>,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            found: Kept::default(),
            ready: Mutex::default(),
            downloads: AtomicU64::new(0),
            installing: AtomicBool::new(false),
            left: AtomicBool::new(false),
            // Started, the window has just been seen.
            presence: Mutex::new(Presence {
                active_at: now_ms(),
                ..Presence::default()
            }),
            plan: Mutex::default(),
            installed: Mutex::default(),
            failed: Mutex::default(),
            check_streak: Mutex::default(),
            download_streak: Mutex::default(),
        }
    }
}

impl Updates {
    /// Keeps what a check found, for its download.
    pub fn offer(&self, release: Arc<dyn Release>) -> Found {
        let (version, notes) = (release.version().to_string(), release.notes().to_string());
        Found {
            id: self.found.keep(release),
            version,
            notes,
        }
    }

    /// The update found as `id` is no longer offered.
    pub fn free(&self, id: u32) {
        self.found.free(id);
    }

    /// A check found nothing: the release was withdrawn, the one downloaded never installs.
    pub fn withdraw(&self) {
        let mut ready = self.ready.lock();
        // A download under way is not kept either.
        self.downloads.fetch_add(1, Ordering::AcqRel);
        *ready = None;
    }

    /// A check failed at `now` (ms): what the window is told of it.
    pub fn check_failed(&self, error: &anyhow::Error, manual: bool, now: i64) -> Trouble {
        Trouble {
            message: trouble_text(error),
            show: self.check_streak.lock().failed(manual, now),
            page: None,
        }
    }

    /// A check went through, whether it found something or not: the series ends.
    pub fn check_succeeded(&self) {
        self.check_streak.lock().succeeded();
    }

    /// A download failed at `now` (ms): what the window is told of it, with the page of the
    /// release for the browser, which gets there when the app cannot.
    pub fn download_failed(&self, error: &anyhow::Error, manual: bool, now: i64) -> Trouble {
        let page = error
            .downcast_ref::<Failure>()
            .map_or_else(|| RELEASES_URL.into(), Failure::page_url);
        Trouble {
            message: trouble_text(error),
            show: self.download_streak.lock().failed(manual, now),
            page: Some(page),
        }
    }

    /// An update is downloaded and checked: the series ends.
    pub fn download_succeeded(&self) {
        self.download_streak.lock().succeeded();
    }

    /// Downloads the update found as `id` (`download`), as the window is told how it went.
    pub async fn downloaded(&self, id: u32, manual: bool) -> Downloaded {
        match self.download(id).await {
            Ok(true) => {
                self.download_succeeded();
                Downloaded::Ready
            }
            Ok(false) => Downloaded::Superseded,
            Err(e) => {
                log::warn!("the update did not download: {e:#}");
                Downloaded::Failed {
                    trouble: self.download_failed(&e, manual, now_ms()),
                }
            }
        }
    }

    /// Downloads the update found as `id` and keeps it ready, unless it already is; an older one
    /// downloaded is no longer offered meanwhile. True when it is the one ready now.
    pub async fn download(&self, id: u32) -> Result<bool> {
        let release = self.found.take(id).ok_or_else(|| {
            anyhow!(tr!(
                "Cette mise à jour n'est plus proposée : recherche-la de nouveau.",
                "This update is no longer offered: look for it again."
            ))
        })?;
        let version = release.version().to_string();
        let started = {
            let mut ready = self.ready.lock();
            if ready.as_ref().is_some_and(|r| r.version == version) {
                return Ok(true);
            }
            *ready = None;
            self.downloads.fetch_add(1, Ordering::AcqRel) + 1
        };
        let package = match release.fetch().await {
            Ok(package) => package,
            Err(first) => retried(&*release, &version, first).await?,
        };
        let mut ready = self.ready.lock();
        // A newer one started meanwhile, or this one was withdrawn.
        if self.downloads.load(Ordering::Acquire) != started {
            return Ok(false);
        }
        *ready = Some(Arc::new(Ready {
            version,
            notes: release.notes().to_string(),
            package,
        }));
        Ok(true)
    }

    pub fn ready(&self) -> Option<Arc<Ready>> {
        self.ready.lock().clone()
    }

    pub fn set_presence(&self, presence: Presence) {
        *self.presence.lock() = presence;
    }

    /// The automatic restart, one tick further (`on`: the setting is on and an update is ready):
    /// planned once the app is at rest, called off when it is not any more, due `WARNING` after.
    pub fn advance(&self, on: bool, busy: Option<Busy>, now: i64) -> Step {
        let mut plan = self.plan.lock();
        let next = step(&plan, on, busy, now);
        match next {
            Step::Warn(at) => plan.at = Some(at),
            Step::Cancel | Step::Restart => plan.at = None,
            Step::Wait => {}
        }
        next
    }

    /// When the automatic restart planned comes, if one is.
    pub fn restart_at(&self) -> Option<i64> {
        self.plan.lock().at
    }

    /// « Plus tard »: the restart planned is called off, and none comes for `POSTPONE`. True
    /// when one was planned.
    pub fn postpone(&self, now: i64) -> bool {
        let mut plan = self.plan.lock();
        plan.not_before = now + POSTPONE;
        plan.at.take().is_some()
    }

    /// What the note of the last install said at this start (`take_note`).
    pub fn started(after: Option<AfterUpdate>) -> Self {
        let u = Self::default();
        match after {
            Some(AfterUpdate::Installed(installed, _)) => *u.installed.lock() = Some(installed),
            Some(AfterUpdate::Failed(version)) => *u.failed.lock() = Some(version),
            None => {}
        }
        u
    }

    /// The update installed since the last start, told once.
    pub fn take_installed(&self) -> Option<Installed> {
        self.installed.lock().take()
    }

    /// The version that did not install at the last try, if any.
    pub fn failed(&self) -> Option<String> {
        self.failed.lock().clone()
    }

    /// The app may restart by itself (`setting`: « Installer les mises à jour automatiquement »):
    /// an update is ready, no install is under way, it is not one that failed to install, and it
    /// installs `unattended` (`unattended()`: no password asked while the user is away).
    pub fn restart_wanted(&self, setting: bool, unattended: bool) -> bool {
        let failed = self.failed.lock().clone();
        setting
            && unattended
            && !self.installing.load(Ordering::Acquire)
            && self
                .ready()
                .is_some_and(|r| failed.as_deref() != Some(r.version.as_str()))
    }
}

/// A download that failed gets a second try: its link may have expired or the release have been
/// published again, so latest.json is read again, and the same version found there is downloaded
/// once more with what it gives. Nothing there, another version, or a reading that fails: the
/// failure of the first try, which is what went wrong.
async fn retried(
    release: &dyn Release,
    version: &str,
    first: anyhow::Error,
) -> Result<Box<dyn Package>> {
    match release.again().await {
        Ok(Some(again)) if again.version() == version => {
            log::warn!(
                "download of {version} failed ({first:#}): latest.json read again, trying once more"
            );
            again.fetch().await
        }
        Ok(_) => Err(first),
        Err(e) => {
            log::warn!("latest.json not read again after a download failed: {e:#}");
            Err(first)
        }
    }
}

/// What the window is told of a failure: the words of a request that was probed, else the error's.
fn trouble_text(error: &anyhow::Error) -> String {
    match error.downcast_ref::<Failure>() {
        Some(failure) => failure.describe(i18n::ui()),
        None => format!("{error:#}"),
    }
}

/// An update the plugin found, and what came with it.
struct Candidate {
    update: Update,
    slot: Slot,
    route: Route,
}

/// A check of the updates: how to reach the network, and where latest.json is.
struct Checker<R: Runtime> {
    app: AppHandle<R>,
    net: Network,
    /// Where latest.json is (`plugins.updater.endpoints`), or a test's server.
    endpoints: Vec<Url>,
    /// How long before a failure that may pass is tried again.
    pause: Duration,
}

impl<R: Runtime> Checker<R> {
    /// A check through the network settings as they are now.
    fn new(app: &AppHandle<R>, settings: &Settings) -> Result<Self> {
        Ok(Self {
            app: app.clone(),
            net: Network::from_settings(settings)?,
            endpoints: configured_endpoints(app)?,
            pause: RETRY_PAUSE,
        })
    }

    /// A release of this check's.
    fn release(self: &Arc<Self>, found: Candidate) -> PluginRelease<R> {
        PluginRelease {
            update: found.update,
            slot: found.slot,
            route: found.route,
            checker: self.clone(),
        }
    }

    /// The plugin's updater for a request that goes `route`.
    fn updater(&self, route: Route, slot: &Slot) -> Result<tauri_plugin_updater::Updater> {
        let (hook_slot, app) = (slot.clone(), self.app.clone());
        let mut builder = self
            .app
            .updater_builder()
            .timeout(TIMEOUT)
            .endpoints(self.endpoints.clone())?
            .header(USER_AGENT, net::user_agent())?
            .on_before_exit(move || {
                // Taken first: the lock is not held while the app stops.
                let hook = hook_slot.lock().take();
                if let Some(hook) = hook {
                    hook();
                }
                // What the plugin's own hook does, which this one replaces: the tray icon goes now.
                app.cleanup_before_exit();
            });
        match self.net.via(route) {
            Via::Nothing => builder = builder.no_proxy(),
            Via::Proxy(proxy) => builder = builder.proxy(proxy.clone()),
            Via::System => {}
        }
        if self.net.insecure_tls {
            builder = builder.configure_client(|c| c.danger_accept_invalid_certs(true));
        }
        Ok(builder.build()?)
    }

    /// One request of the plugin, `route` the way it goes. The plugin does not say what answered
    /// when it fails: the request is made again to learn it, and the failure is a `Failure`.
    async fn once(&self, route: Route) -> Result<Option<Candidate>> {
        let slot = Slot::default();
        let updater = self.updater(route, &slot)?;
        match updater.check().await {
            Ok(update) => Ok(update.map(|update| Candidate {
                update,
                slot,
                route,
            })),
            Err(e) => {
                let Some(endpoint) = self.endpoints.last() else {
                    return Err(e.into());
                };
                // The plugin's last error is that of the last endpoint.
                let failure = probe(
                    endpoint,
                    &self.net,
                    route,
                    "application/json",
                    What::Check,
                    net::chain(&e),
                )
                .await;
                Err(anyhow::Error::new(failure))
            }
        }
    }

    /// A try `route` the way, and if the proxy of the settings is not there (its connection
    /// refused or timed out, its name unknown, no word to its tunnel), one without it: only once
    /// per check. A proxy that answers is never gone around, whatever it says: its refusal is its
    /// company's rule, and the failure says so.
    async fn attempt(&self, route: Route) -> Result<Option<Candidate>> {
        let error = match self.once(route).await {
            Ok(found) => return Ok(found),
            Err(e) => e,
        };
        let mut failure = match error.downcast::<Failure>() {
            Ok(failure) => failure,
            Err(other) => return Err(other),
        };
        if matches!(self.net.via(route), Via::Proxy(_)) && failure.proxy_unreachable() {
            match self.once(Route::Direct).await {
                Ok(found) => {
                    log::warn!("update check through the proxy failed ({failure}), direct worked");
                    return Ok(found);
                }
                Err(e) => {
                    if let Ok(direct) = e.downcast::<Failure>() {
                        failure.direct = Some(Box::new(direct));
                    }
                }
            }
        }
        Err(anyhow::Error::new(failure))
    }

    /// Looks for the update, starting `route` the way: what the settings ask for, then without
    /// the proxy if it is not there, and once more after a pause if the failure may pass.
    async fn find(&self, route: Route) -> Result<Option<Candidate>> {
        match self.attempt(route).await {
            Err(e) if e.downcast_ref::<Failure>().is_some_and(Failure::transient) => {
                log::warn!(
                    "update check failed ({e:#}): trying once more in {:?}",
                    self.pause
                );
                tokio::time::sleep(self.pause).await;
                self.attempt(route).await
            }
            done => done,
        }
    }

    /// `find`, the result kept for its download and told to the window.
    async fn run(self: &Arc<Self>, updates: &Updates, manual: bool) -> Checked {
        match self.find(Route::Proxy).await {
            Ok(Some(found)) => {
                updates.check_succeeded();
                Checked::Found {
                    update: updates.offer(Arc::new(self.release(found))),
                }
            }
            Ok(None) => {
                updates.check_succeeded();
                updates.withdraw();
                Checked::None
            }
            Err(e) => {
                log::warn!("the check for updates failed: {e:#}");
                Checked::Failed {
                    trouble: updates.check_failed(&e, manual, now_ms()),
                }
            }
        }
    }
}

/// Where latest.json is, from the plugin's configuration (`plugins.updater.endpoints`).
fn configured_endpoints<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<Url>> {
    let config = app.config();
    let endpoints: Vec<Url> = config
        .plugins
        .0
        .get("updater")
        .and_then(|c| c.get("endpoints"))
        .and_then(|e| e.as_array())
        .into_iter()
        .flatten()
        .filter_map(|e| e.as_str()?.parse().ok())
        .collect();
    if endpoints.is_empty() {
        bail!("Updater does not have any endpoints set.");
    }
    Ok(endpoints)
}

/// Looks for a newer release through the network settings as they are now, the proxy and the
/// certificate checks of the settings; the update found is kept for its download. None found: the
/// one downloaded, if any, was withdrawn. A failure is told with its cause, and shown (`manual`: the
/// user asked) or kept quiet until it goes on (`Updates::check_failed`).
pub async fn check<R: Runtime>(
    app: &AppHandle<R>,
    settings: &Settings,
    updates: &Updates,
    manual: bool,
) -> Checked {
    match Checker::new(app, settings) {
        Ok(checker) => Arc::new(checker).run(updates, manual).await,
        Err(e) => Checked::Failed {
            trouble: updates.check_failed(&e, manual, now_ms()),
        },
    }
}

/// What the app is doing now that a restart would cut.
pub fn snapshot<R: Runtime>(core: &Core<R>, updates: &Updates, now: i64) -> Snapshot {
    let mut s = Snapshot::default();
    let held = core.held.lock().clone();
    for h in core.agents.read().values() {
        let rt = h.lock();
        if rt.meta.archived {
            continue;
        }
        match rt.meta.status {
            AgentStatus::Running => s.turns += 1,
            AgentStatus::Waiting => s.questions += 1,
            _ if held.contains(&rt.meta.id) => s.turns += 1,
            _ => {}
        }
        if rt.setup.is_some() {
            s.setups += 1;
        }
        // What runs in the background lives in the process: with none, it is gone already.
        if rt.proc.is_some() {
            s.background += rt.meta.plan.as_ref().map_or(0, |p| p.running_count());
        }
    }
    s.validations = core
        .tickets
        .read()
        .iter()
        .filter(|t| t.step.is_some())
        .count();
    s.works = core.works.load(Ordering::Acquire);
    let terminals = core.pty.activity();
    s.launches = terminals.launches;
    s.terminal_quiet = terminals.shell_io_at.map_or(i64::MAX, |at| now - at);
    s.unsaved = core.unsaved.load(Ordering::Acquire);
    let presence = *updates.presence.lock();
    s.testing = presence.testing;
    s.modal = presence.modal;
    s.inactive = now - presence.active_at;
    s.visible = window_visible(&core.app);
    s
}

/// The main window is on screen: shown, not minimized.
fn window_visible<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

/// Runs `f` on a thread of its own and waits for it `limit` at most: true when it ended in time.
/// The calling thread waits meanwhile, so it is never one of the async runtime's.
pub fn bounded(f: impl FnOnce() + Send + 'static, limit: Duration) -> bool {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        f();
        let _ = tx.send(());
    });
    // A panic drops the sender: over at once, not in time.
    rx.recv_timeout(limit).is_ok()
}

/// Installs `package`, `stop` run first, `limit` at most; then `leave` restarts the app on the
/// new version with `relaunch`, or closes it (the Windows installer ends the app itself before
/// that). An install that fails before the stop leaves the app as it was, its error returned;
/// one that fails after it restarts the app as it was (or closes it, without `relaunch`).
pub fn apply(
    package: &dyn Package,
    relaunch: bool,
    stop: Hook,
    limit: Duration,
    leave: impl FnOnce(bool),
) -> Result<()> {
    let stopped = Arc::new(AtomicBool::new(false));
    let before_exit: Hook = {
        let stopped = stopped.clone();
        Arc::new(move || {
            // Once, whoever calls it.
            if stopped.swap(true, Ordering::AcqRel) {
                return;
            }
            let stop = stop.clone();
            if !bounded(move || stop(), limit) {
                log::warn!("the app took over {limit:?} to stop: the update installs all the same");
            }
        })
    };
    let installed = package.install(relaunch, before_exit);
    if let Err(e) = &installed {
        if !stopped.load(Ordering::Acquire) {
            return installed;
        }
        log::error!("update not installed once the app stopped: {e:#}");
    }
    leave(relaunch);
    installed
}

/// The update the app stopped to install, told to the window of its next start.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub version: String,
    pub notes: String,
}

/// How the window was when the app stopped for an update: the app restarted for it brings it
/// back so, neither shown nor taking the focus when it was not.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Window {
    /// On screen and in front (or nothing to go by): shown and focused.
    #[default]
    Front,
    /// On screen behind another app's window: shown, not focused.
    Behind,
    /// Minimized: shown then minimized again, its button in the taskbar or its window in the Dock.
    Minimized,
    /// Closed to the tray: left so.
    Hidden,
}

/// The note written before an install (`install_ready`), read at the next start (`take_note`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Note {
    version: String,
    notes: String,
    /// Not in a note written before it was noted: in front.
    #[serde(default)]
    window: Window,
}

/// What the note of the last install says at this start.
#[derive(Debug, Clone, PartialEq)]
pub enum AfterUpdate {
    /// The app runs the version it stopped for, its window to be as it was.
    Installed(Installed, Window),
    /// It still runs another one: that version did not install.
    Failed(String),
}

impl AfterUpdate {
    /// How the window comes back at this start.
    pub fn window(after: &Option<Self>) -> Window {
        match after {
            Some(Self::Installed(_, window)) => *window,
            _ => Window::Front,
        }
    }
}

/// Installs the update ready, the app stopped cleanly first (`Core::shutdown`: its agents and
/// terminals stop, its state is saved) and the update noted for the next start (`take_note`),
/// with how the `window` is now. With `relaunch`, the app starts again on it, else it stays
/// closed: `leave` restarts or closes it once the update is in place (`apply`). False when there
/// is nothing to install, or an install is under way already; an error when it failed with the
/// app still running.
pub fn install_ready<R: Runtime>(
    core: &Arc<Core<R>>,
    updates: &Updates,
    relaunch: bool,
    window: Window,
    leave: impl FnOnce(bool),
) -> Result<bool> {
    let Some(ready) = updates.ready() else {
        return Ok(false);
    };
    if updates.installing.swap(true, Ordering::AcqRel) {
        return Ok(false);
    }
    let note = core.data.update_note_file();
    let noted = serde_json::to_vec(&Note {
        version: ready.version.clone(),
        notes: ready.notes.clone(),
        window,
    })
    .map_err(anyhow::Error::from)
    .and_then(|bytes| Ok(crate::paths::write_atomic(&note, &bytes)?));
    if let Err(e) = noted {
        log::warn!("cannot note the update installed: {e:#}");
    }
    let stop: Hook = {
        let core = core.clone();
        Arc::new(move || core.shutdown())
    };
    log::info!(
        "installing Escouade {} (relaunch: {relaunch})",
        ready.version
    );
    let installed = apply(&*ready.package, relaunch, stop, STOP_LIMIT, |relaunch| {
        // Leaving may never return (macOS: restarting from another thread waits for the app's
        // end): told first.
        updates.left.store(true, Ordering::Release);
        leave(relaunch)
    });
    if installed.is_err() {
        // The app still runs: it is tried again by hand (« Réessayer »), or when the app closes,
        // never by itself again.
        updates.installing.store(false, Ordering::Release);
        let _ = std::fs::remove_file(&note);
        *updates.failed.lock() = Some(ready.version.clone());
        core.hub.emit(UiEvent::UpdateFailed {
            version: ready.version.clone(),
        });
    }
    installed.map(|()| true)
}

/// Why a restart waits: `n` files of the editor are not saved.
fn unsaved_first(lang: Lang, n: usize) -> String {
    tr_n_in!(
        lang,
        n,
        "Enregistre d'abord tes fichiers : {n} fichier n'est pas enregistré.",
        "Enregistre d'abord tes fichiers : {n} fichiers ne sont pas enregistrés.",
        "Save your files first: {n} file isn’t saved.",
        "Save your files first: {n} files aren’t saved."
    )
}

/// The system notification of an automatic restart: its title, its text, its button.
fn restart_toast(lang: Lang, version: &str) -> [String; 3] {
    [
        tr_in!(
            lang,
            "Escouade redémarre pour se mettre à jour",
            "Escouade is restarting to update"
        ),
        tr_in!(
            lang,
            "La version {version} s'installe dans 30 secondes.",
            "Version {version} installs in 30 seconds."
        ),
        tr_in!(lang, "Plus tard", "Later"),
    ]
}

/// « Redémarrer maintenant », or the automatic restart: the update ready installs and the app
/// starts again on it, its `window` as it is now. Never while the editor holds unsaved files.
pub fn restart<R: Runtime>(
    core: &Arc<Core<R>>,
    updates: &Updates,
    window: Window,
    leave: impl FnOnce(bool),
) -> Result<()> {
    match core.unsaved.load(Ordering::Acquire) {
        0 => {}
        n => bail!(unsaved_first(i18n::ui(), n)),
    }
    if !install_ready(core, updates, true, window, leave)? {
        bail!(tr!(
            "Aucune mise à jour n'est prête : recherche-la de nouveau.",
            "No update is ready: look for it again."
        ));
    }
    Ok(())
}

/// The note of the last install (`install_ready`), taken once: the app runs its version (the
/// window is told of it), or still another one (that version did not install).
pub fn take_note(data: &DataDir, version: &str) -> Option<AfterUpdate> {
    let file = data.update_note_file();
    let bytes = std::fs::read(&file).ok()?;
    let _ = std::fs::remove_file(&file);
    let note: Note = serde_json::from_slice(&bytes).ok()?;
    Some(if note.version == version {
        AfterUpdate::Installed(
            Installed {
                version: note.version,
                notes: note.notes,
            },
            note.window,
        )
    } else {
        AfterUpdate::Failed(note.version)
    })
}

/// What « Quitter » did (`closing`).
#[derive(Debug, PartialEq, Eq)]
pub enum Closing {
    /// The app stopped for the update ready, which `leave` closed.
    Installed,
    /// The app is to close as usual: nothing to install, or an install that failed with the app
    /// still running.
    Close,
    /// An install under way still runs after the wait: it ends the app itself, nothing else may.
    Wait,
}

/// « Quitter »: the update ready installs without the app starting again. One installing
/// already (« Redémarrer maintenant », the automatic restart) ends the app itself: waited for,
/// `wait` at most, never closed under it.
pub fn closing<R: Runtime>(
    core: &Arc<Core<R>>,
    updates: &Updates,
    wait: Duration,
    leave: impl FnOnce(bool),
) -> Closing {
    match install_ready(core, updates, false, Window::Front, leave) {
        Ok(true) => return Closing::Installed,
        Ok(false) => {}
        Err(e) => {
            log::error!("update not installed when closing: {e:#}");
            return Closing::Close;
        }
    }
    // Nothing to install, or one installing already, which ends the app itself.
    let until = std::time::Instant::now() + wait;
    while updates.installing.load(Ordering::Acquire) {
        if std::time::Instant::now() >= until {
            return Closing::Wait;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Not installing (any more): it failed with the app still running, which closes as usual.
    Closing::Close
}

/// Restarts or closes the app once its update is in place, or after an install that failed once
/// the app had stopped. On Windows the plugin's hook cleaned the app up before the installer:
/// no Tauri API may run any more, the process restarts or exits by itself.
fn leave<R: Runtime>(app: &AppHandle<R>, env: &tauri::Env, relaunch: bool) {
    if cfg!(windows) {
        if relaunch {
            tauri::process::restart(env);
        }
        std::process::exit(0);
    }
    if relaunch {
        app.restart();
    }
    app.exit(0);
}

/// How the main window is now.
fn window_now<R: Runtime>(app: &AppHandle<R>) -> Window {
    match app.get_webview_window("main") {
        Some(w) => window_of(
            w.is_visible().unwrap_or(false),
            w.is_minimized().unwrap_or(false),
            w.is_focused().unwrap_or(false),
            cfg!(target_os = "macos"),
        ),
        None => Window::Front,
    }
}

/// How a window `visible` (shown, or closed to the tray), `minimized` and `focused` is noted. On
/// `macos`, a window minimized to the Dock is not visible (`NSWindow.isVisible`), though not in the
/// tray: minimized wins there.
fn window_of(visible: bool, minimized: bool, focused: bool, macos: bool) -> Window {
    match (visible, minimized, focused) {
        (_, true, _) if macos => Window::Minimized,
        (true, false, true) => Window::Front,
        (true, false, false) => Window::Behind,
        (true, true, _) => Window::Minimized,
        (false, _, _) => Window::Hidden,
    }
}

/// « Redémarrer maintenant », or the automatic restart, off the async runtime (the stop and the
/// install block).
pub async fn restart_now<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let core = app.state::<Arc<Core<R>>>().inner().clone();
        let env = app.env();
        let window = window_now(&app);
        restart(&core, &app.state::<Updates>(), window, |relaunch| {
            leave(&app, &env, relaunch)
        })
    })
    .await?
}

/// How long « Quitter » waits for an install under way to end the app.
const QUIT_WAIT: Duration = Duration::from_secs(120);

/// « Quitter »: the app stops; with an update ready, it installs silently and the app stays
/// closed (`closing`).
pub fn quit<R: Runtime>(app: &AppHandle<R>) {
    let core = app.state::<Arc<Core<R>>>().inner().clone();
    let updates = app.state::<Updates>();
    if updates.ready().is_none() && !updates.installing.load(Ordering::Acquire) {
        core.shutdown();
        app.exit(0);
        return;
    }
    // The window goes at once. The install blocks, and on macOS may need the main thread (an
    // administrator's password asked for): it runs apart.
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let env = app.env();
        let closed = closing(&core, &app.state::<Updates>(), QUIT_WAIT, |relaunch| {
            leave(&app, &env, relaunch)
        });
        if closed == Closing::Close {
            core.shutdown();
            app.exit(0);
        }
    });
}

/// The update installs without asking anything: on macOS, the app's bundle and its folder can
/// be written without an administrator's password, which the plugin would ask for on the main
/// thread (frozen until answered, the user maybe away).
fn unattended() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::env::current_exe()
            .ok()
            .as_deref()
            .and_then(app_bundle)
            .is_some_and(|b| writable(b) && b.parent().is_some_and(writable))
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// How long the end of the app waits for an update to install there (macOS).
#[cfg(target_os = "macos")]
const EXIT_LIMIT: Duration = Duration::from_secs(60);

/// The `.app` bundle the executable at `exe` runs from (`…/Escouade.app/Contents/MacOS/escouade`).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn app_bundle(exe: &Path) -> Option<&Path> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let named = |p: &Path, name: &str| p.file_name().is_some_and(|n| n == name);
    let is_app = bundle.extension().is_some_and(|e| e == "app");
    (named(macos, "MacOS") && named(contents, "Contents") && is_app).then_some(bundle)
}

/// The file or folder at `p` can be written by the app.
#[cfg(target_os = "macos")]
fn writable(p: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(p.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: `c` is a valid NUL-terminated path for the call's duration.
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

/// At the app's end, an install under way (« Redémarrer maintenant », the automatic restart) is
/// waited for, `limit` at most, rather than ended under it (on macOS, between the two renames of
/// the bundle): until it fails with the app still running, or has the app leave. None when there
/// is nothing to wait for: none under way, or one not `unattended` (`unattended()`), whose bundle
/// the plugin only moves on the main thread, which the app's end holds. Else whether the app
/// stopped for it, not to be stopped again.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn wait_install<R: Runtime>(
    core: &Core<R>,
    updates: &Updates,
    unattended: bool,
    limit: Duration,
) -> Option<bool> {
    if !unattended || !updates.installing.load(Ordering::Acquire) {
        return None;
    }
    let until = std::time::Instant::now() + limit;
    while updates.installing.load(Ordering::Acquire) && !updates.left.load(Ordering::Acquire) {
        if std::time::Instant::now() >= until {
            log::warn!(
                "the update under way took over {limit:?} to install: the app ends all the same"
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Some(core.quitting.load(Ordering::Acquire))
}

/// macOS: Cmd+Q, the app menu's and the Dock's « Quitter » end the app with no request first,
/// right at `RunEvent::Exit`: the update ready installs there, without the app starting again,
/// when its bundle can be replaced without an administrator's password (the plugin would ask for
/// it on the main thread, which waits here). One installing already is waited for, unless it
/// needs an administrator (`wait_install`). True when the app stopped for it.
#[cfg(target_os = "macos")]
pub fn install_at_exit<R: Runtime>(app: &AppHandle<R>) -> bool {
    let updates = app.state::<Updates>();
    let core = app.state::<Arc<Core<R>>>().inner().clone();
    let unattended = unattended();
    if let Some(stopped) = wait_install(&core, &updates, unattended, EXIT_LIMIT) {
        return stopped;
    }
    if updates.ready().is_none() {
        return false;
    }
    if !unattended {
        log::info!("update left for a restart: replacing the app needs an administrator");
        return false;
    }
    let (a, c) = (app.clone(), core.clone());
    bounded(
        move || {
            let updates = a.state::<Updates>();
            if let Err(e) = install_ready(&c, &updates, false, Window::Front, |_| {}) {
                log::error!("update not installed when closing: {e:#}");
            }
        },
        EXIT_LIMIT,
    );
    core.quitting.load(Ordering::Acquire)
}

/// « Plus tard »: the automatic restart planned is called off, and none comes for a while.
pub fn postpone<R: Runtime>(app: &AppHandle<R>) {
    if app.state::<Updates>().postpone(now_ms()) {
        app.state::<Arc<Core<R>>>()
            .hub
            .emit(UiEvent::UpdateRestart { at: None });
    }
}

/// Restarts the app by itself for the update downloaded, the setting on, once it is at rest,
/// the user warned `WARNING` before (a system notification, the status bar's countdown).
pub fn watch<R: Runtime>(app: AppHandle<R>) {
    // Sandboxed runs (end-to-end tests, demos) never restart under the one driving them.
    if crate::paths::sandbox_dir().is_some() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(TICK).await;
            tick(&app).await;
        }
    });
}

async fn tick<R: Runtime>(app: &AppHandle<R>) {
    let core = app.state::<Arc<Core<R>>>().inner().clone();
    let updates = app.state::<Updates>();
    let now = now_ms();
    let ready = updates.ready();
    let on = updates.restart_wanted(core.settings.read().auto_update, unattended());
    let busy = if on {
        busy(&snapshot(&core, &updates, now))
    } else {
        None
    };
    match updates.advance(on, busy, now) {
        Step::Wait => {}
        Step::Warn(at) => {
            core.hub.emit(UiEvent::UpdateRestart { at: Some(at) });
            // Whatever « Notifications système » says: it is not an agent's notification, and
            // the window may be out of sight.
            let version = ready.map(|r| r.version.clone()).unwrap_or_default();
            let a = app.clone();
            let [title, body, later] = restart_toast(i18n::ui(), &version);
            crate::notify::toast_with_button(app, &title, &body, &later, move |later| {
                if later {
                    postpone(&a);
                } else {
                    crate::notify::show_main(&a);
                }
            });
        }
        Step::Cancel => core.hub.emit(UiEvent::UpdateRestart { at: None }),
        // The countdown stays on screen while the app stops.
        Step::Restart => {
            if let Err(e) = restart_now(app.clone()).await {
                // Tried again later, not at every tick.
                log::error!("automatic restart for the update failed: {e:#}");
                updates.postpone(now_ms());
                core.hub.emit(UiEvent::UpdateRestart { at: None });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_tests::harness;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use tokio::sync::Notify;

    /// `f` within 5 s, or the test fails: a regression never hangs it.
    async fn within<T>(what: &str, f: impl Future<Output = T>) -> T {
        tokio::time::timeout(Duration::from_secs(5), f)
            .await
            .unwrap_or_else(|_| panic!("timed out: {what}"))
    }

    #[test]
    fn what_is_found_is_kept_until_taken_or_freed_each_under_its_own_id() {
        let kept = Kept::<String>::default();
        let a = kept.keep("0.2.0".into());
        let b = kept.keep("0.2.0".into());
        assert_ne!(a, b);
        assert_eq!(kept.take(a).as_deref(), Some("0.2.0"));
        assert_eq!(kept.take(a), None);
        kept.free(b);
        assert_eq!(kept.take(b), None);
        // Freed twice, or never kept: nothing happens.
        kept.free(b);
        kept.free(99);
    }

    // ---------- at rest ----------

    /// The app at rest: nothing under way, no terminal used, the user away for 5 minutes,
    /// window on screen.
    fn rest() -> Snapshot {
        Snapshot {
            inactive: AWAY,
            terminal_quiet: i64::MAX,
            visible: true,
            ..Snapshot::default()
        }
    }

    #[test]
    fn the_app_is_at_rest_only_when_a_restart_would_cut_nothing() {
        assert_eq!(busy(&rest()), None);
        let with = |f: fn(&mut Snapshot)| {
            let mut s = rest();
            f(&mut s);
            s
        };
        let cases = [
            (with(|s| s.turns = 1), Busy::Turn),
            (with(|s| s.questions = 1), Busy::Question),
            (with(|s| s.setups = 1), Busy::Setup),
            (with(|s| s.validations = 1), Busy::Validation),
            (with(|s| s.works = 1), Busy::Work),
            (with(|s| s.background = 1), Busy::Background),
            (with(|s| s.launches = 1), Busy::Launch),
            (with(|s| s.terminal_quiet = AWAY - 1), Busy::Terminal),
            (with(|s| s.testing = true), Busy::Testing),
            (with(|s| s.unsaved = 2), Busy::Unsaved),
            (with(|s| s.modal = true), Busy::Modal),
            (with(|s| s.inactive = AWAY - 1), Busy::Activity),
        ];
        for (s, why) in cases {
            assert_eq!(busy(&s), Some(why), "{s:?}");
            // Window hidden or not, what is under way blocks; only the user's presence does not
            // count once the window is not on screen.
            let hidden = Snapshot {
                visible: false,
                ..s.clone()
            };
            let expected = (why != Busy::Activity).then_some(why);
            assert_eq!(busy(&hidden), expected, "{hidden:?}");
        }
        // A terminal quiet for 5 minutes blocks nothing.
        assert_eq!(busy(&with(|s| s.terminal_quiet = AWAY)), None);
        // Used a minute ago, the window hidden since: at rest.
        assert_eq!(
            busy(&Snapshot {
                inactive: 60_000,
                visible: false,
                ..rest()
            }),
            None
        );
    }

    #[test]
    fn an_automatic_restart_is_warned_first_and_called_off_when_the_app_gets_busy() {
        let u = Updates::default();
        let t = 1_000_000;
        assert_eq!(u.advance(true, Some(Busy::Turn), t), Step::Wait);
        assert_eq!(u.advance(true, None, t), Step::Warn(t + WARNING));
        // Warned: it comes once the warning is over, not before.
        assert_eq!(u.advance(true, None, t + WARNING - 1), Step::Wait);
        assert_eq!(
            u.advance(true, Some(Busy::Activity), t + 5_000),
            Step::Cancel
        );
        assert_eq!(u.advance(true, Some(Busy::Activity), t + 6_000), Step::Wait);
        // At rest again: warned again, the full delay.
        assert_eq!(
            u.advance(true, None, t + 10_000),
            Step::Warn(t + 10_000 + WARNING)
        );
        assert_eq!(u.advance(true, None, t + 10_000 + WARNING), Step::Restart);
        assert_eq!(
            u.advance(true, None, t + 10_000 + WARNING),
            Step::Warn(t + 10_000 + 2 * WARNING)
        );
    }

    #[test]
    fn with_the_setting_off_or_nothing_ready_no_restart_comes() {
        let u = Updates::default();
        let t = 1_000_000;
        assert_eq!(u.advance(false, None, t), Step::Wait);
        assert_eq!(u.advance(true, None, t), Step::Warn(t + WARNING));
        assert_eq!(u.advance(false, None, t + 1_000), Step::Cancel);
        assert_eq!(u.advance(false, None, t + WARNING), Step::Wait);
    }

    #[test]
    fn later_holds_off_the_automatic_restart_for_a_while() {
        let u = Updates::default();
        let t = 1_000_000;
        assert_eq!(u.advance(true, None, t), Step::Warn(t + WARNING));
        assert!(u.postpone(t + 1_000));
        assert!(!u.postpone(t + 2_000));
        assert_eq!(u.advance(true, None, t + WARNING), Step::Wait);
        assert_eq!(u.advance(true, None, t + 2_000 + POSTPONE - 1), Step::Wait);
        assert_eq!(
            u.advance(true, None, t + 2_000 + POSTPONE),
            Step::Warn(t + 2_000 + POSTPONE + WARNING)
        );
    }

    #[tokio::test]
    async fn the_app_restarts_by_itself_only_for_an_update_ready_that_did_not_fail_before() {
        let u = Updates::default();
        assert!(!u.restart_wanted(true, true));
        ready(&u, Install::Ok).await;
        assert!(u.restart_wanted(true, true));
        assert!(!u.restart_wanted(false, true));
        // The last try of 1.6.0 did not install: no automatic restart for it.
        let failed = Updates::started(Some(AfterUpdate::Failed("1.6.0".into())));
        assert_eq!(failed.failed().as_deref(), Some("1.6.0"));
        ready(&failed, Install::Ok).await;
        assert!(!failed.restart_wanted(true, true));
        // A newer one does.
        let r = Arc::new(release("1.7.0"));
        let id = failed.offer(r).id;
        failed.download(id).await.unwrap();
        assert!(failed.restart_wanted(true, true));
    }

    #[tokio::test]
    async fn an_update_that_would_ask_for_a_password_never_restarts_the_app_by_itself() {
        let u = Updates::default();
        ready(&u, Install::Ok).await;
        // macOS, the app's folder writable by an administrator only: the plugin would ask for the
        // password on the main thread, the user maybe away.
        assert!(!u.restart_wanted(true, false));
        assert!(u.restart_wanted(true, true));
        // By hand it still installs, password and all.
        let h = harness("upd-password-by-hand");
        restart(&h.core, &u, Window::Front, |_| {}).unwrap();
    }

    #[tokio::test]
    async fn an_install_that_fails_with_the_app_running_is_remembered_and_told() {
        let h = harness("upd-fails-told");
        let u = Updates::default();
        ready(&u, Install::FailsBefore).await;
        assert!(u.restart_wanted(true, true));
        assert!(restart(&h.core, &u, Window::Front, |_| panic!("left")).is_err());
        // No automatic restart for it any more, in this run either.
        assert_eq!(u.failed().as_deref(), Some("1.6.0"));
        assert!(!u.restart_wanted(true, true));
        // The window is told, to offer « Réessayer ».
        assert!(h
            .events
            .lock()
            .iter()
            .any(|e| e["type"] == "updateFailed" && e["version"] == "1.6.0"));
        // Still ready: by hand, it is tried again.
        assert!(u.ready().is_some());
        assert!(restart(&h.core, &u, Window::Front, |_| panic!("left")).is_err());
    }

    #[tokio::test]
    async fn a_version_that_failed_to_install_can_still_be_installed_by_hand() {
        let h = harness("upd-failed-by-hand");
        let u = Updates::started(Some(AfterUpdate::Failed("1.6.0".into())));
        let log = ready(&u, Install::Ok).await;
        restart(&h.core, &u, Window::Front, |_| {}).unwrap();
        assert_eq!(*log.lock(), ["installer (relaunch: true)"]);
    }

    // ---------- download ----------

    /// What a fake package's install saw and did, in order.
    type Log = Arc<Mutex<Vec<String>>>;

    struct FakeRelease {
        version: String,
        fetches: AtomicUsize,
        /// The download fails.
        broken: AtomicBool,
        /// The download fails with this, a request that was probed.
        refused: Mutex<Option<net::Failure>>,
        /// The download waits for it.
        gate: Option<Arc<Notify>>,
        log: Log,
        /// What the package's install does: like the plugin's, or failing before or after
        /// `before_exit`.
        install: Install,
        /// Told apart in the errors of its download.
        name: &'static str,
        /// What reading latest.json again finds, and how many times it was read.
        again: Mutex<Again>,
        readings: AtomicUsize,
    }

    enum Again {
        Nothing,
        Fails,
        Finds(Arc<FakeRelease>),
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Install {
        Ok,
        FailsBefore,
        FailsAfter,
    }

    fn release(version: &str) -> FakeRelease {
        FakeRelease {
            version: version.into(),
            fetches: AtomicUsize::new(0),
            broken: AtomicBool::new(false),
            refused: Mutex::new(None),
            gate: None,
            log: Log::default(),
            install: Install::Ok,
            name: "first",
            again: Mutex::new(Again::Nothing),
            readings: AtomicUsize::new(0),
        }
    }

    impl Release for FakeRelease {
        fn version(&self) -> &str {
            &self.version
        }

        fn notes(&self) -> &str {
            "- Mises à jour silencieuses"
        }

        fn fetch(&self) -> Fetching<'_> {
            Box::pin(async move {
                self.fetches.fetch_add(1, Ordering::AcqRel);
                if let Some(gate) = &self.gate {
                    gate.notified().await;
                }
                let refused = self.refused.lock().clone();
                if let Some(failure) = refused {
                    return Err(anyhow::Error::new(failure));
                }
                if self.broken.load(Ordering::Acquire) {
                    bail!("Download request failed with status: 502 ({})", self.name);
                }
                Ok(Box::new(FakePackage {
                    log: self.log.clone(),
                    install: self.install,
                }) as Box<dyn Package>)
            })
        }

        fn again(&self) -> Reading<'_> {
            Box::pin(async move {
                self.readings.fetch_add(1, Ordering::AcqRel);
                match &*self.again.lock() {
                    Again::Nothing => Ok(None),
                    Again::Fails => bail!("latest.json: 502"),
                    Again::Finds(release) => Ok(Some(release.clone() as Arc<dyn Release>)),
                }
            })
        }
    }

    struct FakePackage {
        log: Log,
        install: Install,
    }

    impl Package for FakePackage {
        fn install(&self, relaunch: bool, before_exit: Hook) -> Result<()> {
            if self.install == Install::FailsBefore {
                bail!("installer unreadable");
            }
            // Like the plugin: the app's hook first, then the installer.
            before_exit();
            if self.install == Install::FailsAfter {
                bail!("installer not started");
            }
            self.log
                .lock()
                .push(format!("installer (relaunch: {relaunch})"));
            Ok(())
        }
    }

    /// Until `r`'s download has started.
    async fn fetching(r: &FakeRelease) {
        within("the download to start", async {
            while r.fetches.load(Ordering::Acquire) == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
    }

    #[tokio::test]
    async fn a_version_found_is_downloaded_once_and_kept_until_it_installs() {
        let u = Updates::default();
        let r = Arc::new(release("1.6.0"));
        let found = u.offer(r.clone());
        assert_eq!(found.version, "1.6.0");
        assert_eq!(found.notes, "- Mises à jour silencieuses");
        assert!(u.ready().is_none());
        assert!(u.download(found.id).await.unwrap());
        assert_eq!(u.ready().unwrap().version, "1.6.0");
        // Found again by the next check: not downloaded again.
        let again = u.offer(r.clone());
        assert!(u.download(again.id).await.unwrap());
        assert_eq!(r.fetches.load(Ordering::Acquire), 1);
        // Each id downloads once.
        assert!(u.download(found.id).await.is_err());
    }

    #[tokio::test]
    async fn a_failed_download_is_tried_again_at_the_next_check() {
        let u = Updates::default();
        let r = Arc::new(release("1.6.0"));
        r.broken.store(true, Ordering::Release);
        let first = u.offer(r.clone());
        assert!(u.download(first.id).await.is_err());
        assert!(u.ready().is_none());
        r.broken.store(false, Ordering::Release);
        let next = u.offer(r.clone());
        assert!(u.download(next.id).await.unwrap());
        assert_eq!(u.ready().unwrap().version, "1.6.0");
    }

    #[tokio::test]
    async fn a_newer_version_replaces_the_one_downloaded_and_a_withdrawn_one_never_installs() {
        let u = Arc::new(Updates::default());
        let older = u.offer(Arc::new(release("1.6.0")));
        u.download(older.id).await.unwrap();
        // The newer one is downloading: the older one is offered no more.
        let gate = Arc::new(Notify::new());
        let newer = Arc::new(FakeRelease {
            gate: Some(gate.clone()),
            ..release("1.7.0")
        });
        let id = u.offer(newer.clone()).id;
        let downloading = tokio::spawn({
            let u = u.clone();
            async move { u.download(id).await }
        });
        fetching(&newer).await;
        assert!(u.ready().is_none());
        gate.notify_one();
        assert!(within("the download", downloading).await.unwrap().unwrap());
        assert_eq!(u.ready().unwrap().version, "1.7.0");
        // Withdrawn from the server: nothing is ready any more.
        u.withdraw();
        assert!(u.ready().is_none());
    }

    #[tokio::test]
    async fn a_download_that_ends_after_a_withdrawal_is_not_kept() {
        let u = Arc::new(Updates::default());
        let gate = Arc::new(Notify::new());
        let r = Arc::new(FakeRelease {
            gate: Some(gate.clone()),
            ..release("1.6.0")
        });
        let id = u.offer(r.clone()).id;
        let downloading = tokio::spawn({
            let u = u.clone();
            async move { u.download(id).await }
        });
        fetching(&r).await;
        u.withdraw();
        gate.notify_one();
        assert!(!within("the download", downloading).await.unwrap().unwrap());
        assert!(u.ready().is_none());
    }

    // ---------- a download that fails ----------

    /// `first` and `second`, the same version, `second` being what reading latest.json again finds
    /// after `first`'s download failed.
    fn broken_then(second: FakeRelease) -> (Arc<FakeRelease>, Arc<FakeRelease>) {
        let (first, second) = (Arc::new(release("1.6.0")), Arc::new(second));
        first.broken.store(true, Ordering::Release);
        *first.again.lock() = Again::Finds(second.clone());
        (first, second)
    }

    fn named(name: &'static str, version: &str) -> FakeRelease {
        FakeRelease {
            name,
            ..release(version)
        }
    }

    #[tokio::test]
    async fn a_download_that_fails_reads_latest_json_again_and_tries_what_it_gives() {
        let u = Updates::default();
        let (first, second) = broken_then(named("second", "1.6.0"));
        let id = u.offer(first.clone()).id;
        assert!(u.download(id).await.unwrap());
        assert_eq!(first.fetches.load(Ordering::Acquire), 1);
        assert_eq!(first.readings.load(Ordering::Acquire), 1);
        assert_eq!(second.fetches.load(Ordering::Acquire), 1);
        // The one kept is the version found, ready to install.
        assert_eq!(u.ready().unwrap().version, "1.6.0");
    }

    #[tokio::test]
    async fn a_download_tried_again_is_tried_once_only_and_its_failure_is_the_one_told() {
        let u = Updates::default();
        let second = named("second", "1.6.0");
        second.broken.store(true, Ordering::Release);
        let (first, second) = broken_then(second);
        let id = u.offer(first.clone()).id;
        let e = u.download(id).await.unwrap_err();
        assert!(e.to_string().contains("(second)"), "{e}");
        assert_eq!(first.fetches.load(Ordering::Acquire), 1);
        assert_eq!(second.fetches.load(Ordering::Acquire), 1);
        // Latest.json was read once: the second try does not read it again.
        assert_eq!(first.readings.load(Ordering::Acquire), 1);
        assert_eq!(second.readings.load(Ordering::Acquire), 0);
        assert!(u.ready().is_none());
    }

    #[tokio::test]
    async fn a_download_is_not_tried_again_with_another_version_nothing_or_a_reading_that_failed() {
        for again in [
            Again::Finds(Arc::new(named("newer", "1.7.0"))),
            Again::Nothing,
            Again::Fails,
        ] {
            let u = Updates::default();
            let first = Arc::new(release("1.6.0"));
            first.broken.store(true, Ordering::Release);
            let other = match &again {
                Again::Finds(other) => Some(other.clone()),
                _ => None,
            };
            *first.again.lock() = again;
            let id = u.offer(first.clone()).id;
            // The first failure is the one told.
            let e = u.download(id).await.unwrap_err();
            assert!(e.to_string().contains("(first)"), "{e}");
            assert_eq!(first.fetches.load(Ordering::Acquire), 1);
            assert_eq!(first.readings.load(Ordering::Acquire), 1);
            if let Some(other) = other {
                assert_eq!(other.fetches.load(Ordering::Acquire), 0);
            }
            assert!(u.ready().is_none());
        }
    }

    #[tokio::test]
    async fn a_download_that_goes_through_reads_nothing_again() {
        let u = Updates::default();
        let r = Arc::new(release("1.6.0"));
        let id = u.offer(r.clone()).id;
        assert!(u.download(id).await.unwrap());
        assert_eq!(r.readings.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn a_download_withdrawn_while_it_is_read_again_is_not_kept() {
        // The release is withdrawn while the second try downloads: it ends after, and is not kept.
        let gate = Arc::new(Notify::new());
        let (first, second) = broken_then(FakeRelease {
            gate: Some(gate.clone()),
            ..named("second", "1.6.0")
        });
        let u = Arc::new(Updates::default());
        let id = u.offer(first.clone()).id;
        let downloading = tokio::spawn({
            let u = u.clone();
            async move { u.download(id).await }
        });
        fetching(&second).await;
        u.withdraw();
        gate.notify_one();
        assert!(!within("the download", downloading).await.unwrap().unwrap());
        assert!(u.ready().is_none());
    }

    // ---------- the failures told ----------

    /// 5 minutes, the pace of the checks in these tests.
    const EVERY: i64 = 5 * 60_000;

    fn boom() -> anyhow::Error {
        anyhow!("boom")
    }

    #[test]
    fn failures_that_go_on_for_less_than_an_hour_or_are_fewer_than_three_stay_quiet() {
        let u = Updates::default();
        let t = 10_000_000;
        // Six in 25 minutes.
        for i in 0..6 {
            let trouble = u.check_failed(&boom(), false, t + i * EVERY);
            assert!(!trouble.show, "failure {i}");
            assert_eq!(trouble.message, "boom");
            assert_eq!(trouble.page, None);
        }
        // Two failures an hour apart are two: not enough.
        let u = Updates::default();
        assert!(!u.check_failed(&boom(), false, t).show);
        assert!(!u.check_failed(&boom(), false, t + SHOW_AFTER).show);
    }

    #[test]
    fn failures_that_went_on_for_an_hour_are_told_once_and_not_again() {
        let u = Updates::default();
        let t = 10_000_000;
        assert!(!u.check_failed(&boom(), false, t).show);
        assert!(!u.check_failed(&boom(), false, t + STREAK_GAP).show);
        // The third, an hour after the first.
        assert!(u.check_failed(&boom(), false, t + SHOW_AFTER).show);
        // Not told again, however long it goes on.
        for i in 1..=60 {
            assert!(
                !u.check_failed(&boom(), false, t + SHOW_AFTER + i * EVERY)
                    .show,
                "failure {i} after the hour"
            );
        }
    }

    #[test]
    fn failures_every_five_minutes_are_told_when_the_hour_is_up() {
        let u = Updates::default();
        let t = 10_000_000;
        // Twelve failures in 55 minutes: not an hour yet.
        for i in 0..12 {
            assert!(!u.check_failed(&boom(), false, t + i * EVERY).show, "{i}");
        }
        // The thirteenth, an hour after the first.
        assert!(u.check_failed(&boom(), false, t + 12 * EVERY).show);
        assert!(!u.check_failed(&boom(), false, t + 13 * EVERY).show);
    }

    #[test]
    fn a_failure_before_a_sleep_and_two_after_it_are_not_a_series() {
        let u = Updates::default();
        let t = 10_000_000;
        // The last failure of the series is over half an hour old when the next comes (the
        // computer slept): a series that starts again, of one, then of two.
        assert!(!u.check_failed(&boom(), false, t).show);
        assert!(!u.check_failed(&boom(), false, t + 70 * 60_000).show);
        assert!(!u.check_failed(&boom(), false, t + 71 * 60_000).show);
    }

    #[test]
    fn a_series_that_went_quiet_for_more_than_half_an_hour_starts_over_and_may_be_told_again() {
        let u = Updates::default();
        let t = 10_000_000;
        // Exactly half an hour between two failures is not a gap, a millisecond more is.
        u.check_failed(&boom(), false, t);
        u.check_failed(&boom(), false, t + STREAK_GAP);
        assert!(u.check_failed(&boom(), false, t + 2 * STREAK_GAP).show);
        // Told. Then silence for more than the gap: a new series, told again after its own hour.
        let later = t + 2 * STREAK_GAP + STREAK_GAP + 1;
        for i in 0..12 {
            assert!(
                !u.check_failed(&boom(), false, later + i * EVERY).show,
                "{i}"
            );
        }
        assert!(u.check_failed(&boom(), false, later + 12 * EVERY).show);
        // A gap of a millisecond more than the limit ends a series that was about to be told.
        let u = Updates::default();
        for i in 0..12 {
            u.check_failed(&boom(), false, t + i * EVERY);
        }
        let last = t + 11 * EVERY;
        assert!(!u.check_failed(&boom(), false, last + STREAK_GAP + 1).show);
        assert!(
            !u.check_failed(&boom(), false, last + STREAK_GAP + 1 + EVERY)
                .show
        );
    }

    #[test]
    fn a_success_ends_the_series_and_the_next_one_starts_over() {
        let u = Updates::default();
        let t = 10_000_000;
        u.check_failed(&boom(), false, t);
        u.check_failed(&boom(), false, t + EVERY);
        u.check_succeeded();
        // Two more, long after: a new series of two.
        assert!(!u.check_failed(&boom(), false, t + 3 * SHOW_AFTER).show);
        assert!(
            !u.check_failed(&boom(), false, t + 3 * SHOW_AFTER + EVERY)
                .show
        );
        // A series that was told, ended, then begun again, is told again.
        let u = Updates::default();
        for i in 0..3 {
            u.check_failed(&boom(), false, t + i * SHOW_AFTER / 2);
        }
        u.check_succeeded();
        let later = t + 10 * SHOW_AFTER;
        assert!(!u.check_failed(&boom(), false, later).show);
        assert!(!u.check_failed(&boom(), false, later + SHOW_AFTER / 2).show);
        assert!(u.check_failed(&boom(), false, later + SHOW_AFTER).show);
    }

    #[test]
    fn a_failure_the_user_asked_for_is_always_shown() {
        let u = Updates::default();
        let t = 10_000_000;
        assert!(u.check_failed(&boom(), true, t).show);
        assert!(u.check_failed(&boom(), true, t + 1).show);
        // The automatic rule is not spent by it: the series goes on from the first failure.
        for i in 1..12 {
            assert!(!u.check_failed(&boom(), false, t + i * EVERY).show, "{i}");
        }
        assert!(u.check_failed(&boom(), false, t + SHOW_AFTER).show);
        assert!(!u.check_failed(&boom(), false, t + SHOW_AFTER + 1).show);
        assert!(u.check_failed(&boom(), true, t + SHOW_AFTER + 2).show);
    }

    #[test]
    fn the_series_of_the_checks_and_of_the_downloads_are_apart() {
        let u = Updates::default();
        let t = 10_000_000;
        // Downloads refused every 5 minutes while the checks go through: told after an hour.
        for i in 0..12 {
            u.check_succeeded();
            assert!(!u.download_failed(&boom(), false, t + i * EVERY).show);
        }
        assert!(u.download_failed(&boom(), false, t + 12 * EVERY).show);
        // Checks failing do not touch it.
        let u = Updates::default();
        for i in 0..3 {
            u.check_failed(&boom(), false, t + i * SHOW_AFTER / 2);
        }
        assert!(!u.download_failed(&boom(), false, t + SHOW_AFTER).show);
        // Nor does a download that went through end the series of the checks.
        let u = Updates::default();
        for i in 0..12 {
            u.check_failed(&boom(), false, t + i * EVERY);
        }
        u.download_succeeded();
        assert!(u.check_failed(&boom(), false, t + 12 * EVERY).show);
        // And a check that went through, not the downloads'.
        let u = Updates::default();
        for i in 0..12 {
            u.download_failed(&boom(), false, t + i * EVERY);
        }
        u.check_succeeded();
        assert!(u.download_failed(&boom(), false, t + 12 * EVERY).show);
    }

    #[test]
    fn a_failure_of_the_network_is_told_in_its_own_words_and_any_other_in_the_errors() {
        let u = Updates::default();
        let failure = net::Failure {
            what: net::What::Download,
            url: "https://github.com/me/app/releases/download/v1.8.0/app-setup.exe".into(),
            status: Some(404),
            answered: true,
            host: "release-assets.githubusercontent.com".into(),
            proxy: None,
            agent: None,
            page: None,
            detail: String::new(),
            stuck: None,
            trusted: Vec::new(),
            direct: None,
        };
        let told = u.download_failed(&anyhow::Error::new(failure), true, 1);
        assert_eq!(
            told.message,
            "404 Not Found de release-assets.githubusercontent.com"
        );
        // The release's page for the browser, whatever the failure: a download has a page.
        assert_eq!(
            told.page.as_deref(),
            Some("https://github.com/me/app/releases/tag/v1.8.0")
        );
        let other = u.download_failed(&anyhow!("Cette mise à jour n'est plus proposée"), true, 2);
        assert_eq!(other.message, "Cette mise à jour n'est plus proposée");
        assert_eq!(other.page.as_deref(), Some(net::RELEASES_URL));
        // A check has no page to download from.
        let check = u.check_failed(&anyhow::Error::new(refused_check()), true, 3);
        assert_eq!(check.page, None);
        assert!(check.message.starts_with("403 Forbidden de github.com"));
    }

    fn refused_check() -> net::Failure {
        net::Failure {
            what: net::What::Check,
            url: "https://github.com/me/app/releases/latest/download/latest.json".into(),
            status: Some(403),
            answered: true,
            host: "github.com".into(),
            proxy: None,
            agent: None,
            page: None,
            detail: String::new(),
            stuck: None,
            trusted: Vec::new(),
            direct: None,
        }
    }

    #[tokio::test]
    async fn the_window_is_told_how_a_download_ended() {
        let u = Updates::default();
        // A failure, told in the words of the failure, and the series begun.
        let r = Arc::new(release("1.6.0"));
        *r.refused.lock() = Some(refused_check());
        let id = u.offer(r.clone()).id;
        let Downloaded::Failed { trouble } = u.downloaded(id, true).await else {
            panic!("the download should fail");
        };
        assert!(trouble.show);
        assert!(trouble.message.starts_with("403 Forbidden de github.com"));
        // Then it goes through: the series is over, and the window is told so. Three failures
        // after it, long after the first, are a new series of three that has not gone on for an
        // hour (it would be told if the first failure still counted).
        *r.refused.lock() = None;
        let id = u.offer(r.clone()).id;
        assert_eq!(u.downloaded(id, false).await, Downloaded::Ready);
        let t = now_ms() + 2 * SHOW_AFTER;
        for i in 0..3 {
            assert!(!u.download_failed(&boom(), false, t + i * EVERY).show);
        }
        // An id no longer offered is a failure too, not a refusal to answer.
        let Downloaded::Failed { trouble } = u.downloaded(99, false).await else {
            panic!("an id that is gone fails");
        };
        assert!(trouble.message.contains("n'est plus proposée"));
    }

    #[tokio::test]
    async fn a_download_withdrawn_meanwhile_is_told_as_superseded() {
        let u = Arc::new(Updates::default());
        let gate = Arc::new(Notify::new());
        let r = Arc::new(FakeRelease {
            gate: Some(gate.clone()),
            ..release("1.6.0")
        });
        let id = u.offer(r.clone()).id;
        let downloading = tokio::spawn({
            let u = u.clone();
            async move { u.downloaded(id, false).await }
        });
        fetching(&r).await;
        u.withdraw();
        gate.notify_one();
        assert_eq!(
            within("the download", downloading).await.unwrap(),
            Downloaded::Superseded
        );
    }

    #[test]
    fn the_window_reads_what_a_check_and_a_download_found_in_camel_case_tagged_by_kind() {
        use serde_json::json;
        let found = Found {
            id: 3,
            version: "1.8.0".into(),
            notes: "- Nouveau".into(),
        };
        let trouble = Trouble {
            message: "403 Forbidden de github.com".into(),
            show: true,
            page: Some("https://github.com/me/app/releases/tag/v1.8.0".into()),
        };
        fn value<T: Serialize>(v: &T) -> serde_json::Value {
            serde_json::to_value(v).unwrap()
        }
        assert_eq!(
            value(&Checked::Found {
                update: found.clone()
            }),
            json!({ "kind": "found", "update": { "id": 3, "version": "1.8.0", "notes": "- Nouveau" } })
        );
        assert_eq!(value(&Checked::None), json!({ "kind": "none" }));
        assert_eq!(
            value(&Checked::Failed {
                trouble: trouble.clone()
            }),
            json!({ "kind": "failed", "trouble": {
                "message": "403 Forbidden de github.com",
                "show": true,
                "page": "https://github.com/me/app/releases/tag/v1.8.0"
            } })
        );
        assert_eq!(value(&Downloaded::Ready), json!({ "kind": "ready" }));
        assert_eq!(
            value(&Downloaded::Superseded),
            json!({ "kind": "superseded" })
        );
        assert_eq!(
            value(&Downloaded::Failed {
                trouble: Trouble {
                    page: None,
                    ..trouble
                }
            }),
            json!({ "kind": "failed", "trouble": {
                "message": "403 Forbidden de github.com", "show": true, "page": null
            } })
        );
    }

    // ---------- install ----------

    fn package(install: Install) -> (FakePackage, Log) {
        let log = Log::default();
        (
            FakePackage {
                log: log.clone(),
                install,
            },
            log,
        )
    }

    /// A stop that takes `ms`, noted when it is over.
    fn slow_stop(log: &Log, ms: u64) -> Hook {
        let log = log.clone();
        Arc::new(move || {
            std::thread::sleep(Duration::from_millis(ms));
            log.lock().push("stopped".into());
        })
    }

    #[test]
    fn the_installer_starts_once_the_app_has_stopped_then_the_new_version_starts() {
        let (p, log) = package(Install::Ok);
        let left = log.clone();
        apply(
            &p,
            true,
            slow_stop(&log, 100),
            STOP_LIMIT,
            move |relaunch| left.lock().push(format!("leave (relaunch: {relaunch})")),
        )
        .unwrap();
        assert_eq!(
            *log.lock(),
            [
                "stopped",
                "installer (relaunch: true)",
                "leave (relaunch: true)"
            ]
        );
    }

    #[test]
    fn a_stop_that_hangs_does_not_keep_the_update_from_installing() {
        let (p, log) = package(Install::Ok);
        let started = std::time::Instant::now();
        apply(
            &p,
            true,
            slow_stop(&log, 5_000),
            Duration::from_millis(50),
            |_| {},
        )
        .unwrap();
        assert!(started.elapsed() < Duration::from_secs(4));
        assert_eq!(*log.lock(), ["installer (relaunch: true)"]);
    }

    #[test]
    fn an_install_that_fails_before_the_stop_leaves_the_app_running() {
        let (p, log) = package(Install::FailsBefore);
        let left = Arc::new(AtomicBool::new(false));
        let l = left.clone();
        let res = apply(&p, true, slow_stop(&log, 0), STOP_LIMIT, move |_| {
            l.store(true, Ordering::Release)
        });
        assert!(res.is_err());
        assert!(log.lock().is_empty());
        assert!(!left.load(Ordering::Acquire));
    }

    #[test]
    fn an_install_that_fails_once_the_app_stopped_starts_it_again() {
        let (p, log) = package(Install::FailsAfter);
        let left = log.clone();
        let res = apply(&p, true, slow_stop(&log, 0), STOP_LIMIT, move |relaunch| {
            left.lock().push(format!("leave (relaunch: {relaunch})"))
        });
        assert!(res.is_err());
        assert_eq!(*log.lock(), ["stopped", "leave (relaunch: true)"]);
    }

    /// An update ready to install in `u`, its package's install as `install` says.
    async fn ready(u: &Updates, install: Install) -> Log {
        let r = Arc::new(FakeRelease {
            install,
            ..release("1.6.0")
        });
        let log = r.log.clone();
        let id = u.offer(r).id;
        u.download(id).await.unwrap();
        log
    }

    #[tokio::test]
    async fn an_update_installs_once_the_app_stopped_cleanly_and_its_next_start_tells_of_it() {
        let h = harness("upd-restart");
        let core = h.core.clone();
        let u = Updates::default();
        let log = ready(&u, Install::Ok).await;
        // Not saved yet: the stop saves it.
        core.ui.write().view = "stats".into();
        let seen = log.clone();
        let c = core.clone();
        restart(&core, &u, Window::Behind, move |relaunch| {
            // What the installer found: the app stopped, its state saved, the update noted.
            let state = std::fs::read_to_string(c.data.state_file()).unwrap_or_default();
            seen.lock().push(format!(
                "leave (relaunch: {relaunch}, stopped: {}, saved: {}, noted: {})",
                c.quitting.load(Ordering::Acquire),
                state.contains("\"stats\""),
                c.data.update_note_file().exists()
            ))
        })
        .unwrap();
        assert_eq!(
            *log.lock(),
            [
                "installer (relaunch: true)",
                "leave (relaunch: true, stopped: true, saved: true, noted: true)"
            ]
        );
        // The window behind another app's: it comes back so.
        let after = take_note(&core.data, "1.6.0");
        assert_eq!(
            after,
            Some(AfterUpdate::Installed(
                Installed {
                    version: "1.6.0".into(),
                    notes: "- Mises à jour silencieuses".into()
                },
                Window::Behind
            ))
        );
        assert_eq!(AfterUpdate::window(&after), Window::Behind);
        // Told once.
        assert_eq!(take_note(&core.data, "1.6.0"), None);
        let u = Updates::started(after);
        assert_eq!(
            u.take_installed().map(|i| i.version).as_deref(),
            Some("1.6.0")
        );
        assert_eq!(u.take_installed(), None);
        assert_eq!(u.failed(), None);
    }

    #[tokio::test]
    async fn closing_the_app_installs_the_update_without_starting_it_again() {
        let h = harness("upd-quit");
        let u = Updates::default();
        let log = ready(&u, Install::Ok).await;
        let left = log.clone();
        let closed = closing(&h.core, &u, Duration::from_secs(5), move |relaunch| {
            left.lock().push(format!("leave (relaunch: {relaunch})"))
        });
        assert_eq!(closed, Closing::Installed);
        assert!(h.core.quitting.load(Ordering::Acquire));
        assert_eq!(
            *log.lock(),
            ["installer (relaunch: false)", "leave (relaunch: false)"]
        );
        // Started a new version: the next start finds it in front.
        assert_eq!(
            AfterUpdate::window(&take_note(&h.core.data, "1.6.0")),
            Window::Front
        );
    }

    #[tokio::test]
    async fn closing_the_app_during_an_install_waits_for_it_never_closing_it_under_it() {
        let h = harness("upd-quit-installing");
        let u = Arc::new(Updates::default());
        let log = ready(&u, Install::Ok).await;
        // « Redémarrer maintenant » under way.
        u.installing.store(true, Ordering::Release);
        assert_eq!(
            closing(&h.core, &u, Duration::from_millis(100), |_| panic!("left")),
            Closing::Wait
        );
        assert!(!h.core.quitting.load(Ordering::Acquire));
        // That install fails with the app still running: the app then closes as usual.
        let failing = {
            let u = u.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                u.installing.store(false, Ordering::Release);
            })
        };
        let started = std::time::Instant::now();
        assert_eq!(
            closing(&h.core, &u, Duration::from_secs(5), |_| panic!("left")),
            Closing::Close
        );
        assert!(started.elapsed() >= Duration::from_millis(150));
        failing.join().unwrap();
        // Nothing installed by the closing itself, nor stopped under the install.
        assert!(log.lock().is_empty());
        assert!(!h.core.quitting.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn the_end_of_the_app_waits_for_an_install_under_way_never_ending_under_it() {
        let h = harness("upd-exit-installing");
        let u = Arc::new(Updates::default());
        // None under way: nothing to wait for.
        assert_eq!(
            wait_install(&h.core, &u, true, Duration::from_secs(5)),
            None
        );
        // « Redémarrer maintenant » under way, the app's folder writable by an administrator only:
        // the plugin moves the bundle on the main thread, which the app's end holds. Nothing moves
        // under it, and waiting would only freeze it.
        u.installing.store(true, Ordering::Release);
        let started = std::time::Instant::now();
        assert_eq!(
            wait_install(&h.core, &u, false, Duration::from_secs(5)),
            None
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        // Under way, still after the wait: the app was not stopped for it.
        let started = std::time::Instant::now();
        assert_eq!(
            wait_install(&h.core, &u, true, Duration::from_millis(100)),
            Some(false)
        );
        assert!(started.elapsed() >= Duration::from_millis(100));
        // It fails with the app still running: the app's end stops it as usual.
        let failing = {
            let u = u.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                u.installing.store(false, Ordering::Release);
            })
        };
        let started = std::time::Instant::now();
        assert_eq!(
            wait_install(&h.core, &u, true, Duration::from_secs(5)),
            Some(false)
        );
        assert!(started.elapsed() >= Duration::from_millis(150));
        failing.join().unwrap();
        // It installs and has the app leave, which never returns (macOS: `AppHandle::restart` from
        // another thread waits for the app's end): over, the app stopped for it.
        let log = ready(&u, Install::Ok).await;
        let (left_tx, left_rx) = std::sync::mpsc::channel();
        let (end_tx, end_rx) = std::sync::mpsc::channel::<()>();
        let installing = {
            let (core, u) = (h.core.clone(), u.clone());
            std::thread::spawn(move || {
                install_ready(&core, &u, true, Window::Front, move |_| {
                    left_tx.send(()).unwrap();
                    let _ = end_rx.recv();
                })
            })
        };
        left_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let started = std::time::Instant::now();
        assert_eq!(
            wait_install(&h.core, &u, true, Duration::from_secs(5)),
            Some(true)
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(*log.lock(), ["installer (relaunch: true)"]);
        end_tx.send(()).unwrap();
        assert!(installing.join().unwrap().unwrap());
    }

    #[tokio::test]
    async fn nothing_installs_without_an_update_downloaded() {
        let h = harness("upd-none");
        let u = Updates::default();
        assert!(!install_ready(&h.core, &u, true, Window::Front, |_| panic!("left")).unwrap());
        assert!(restart(&h.core, &u, Window::Front, |_| panic!("left")).is_err());
        assert_eq!(
            closing(&h.core, &u, Duration::from_secs(5), |_| panic!("left")),
            Closing::Close
        );
        assert!(!h.core.quitting.load(Ordering::Acquire));
    }

    #[test]
    fn an_update_says_in_english_what_holds_it_back_and_when_it_restarts() {
        use crate::i18n::Lang::{En, Fr};
        assert_eq!(
            [unsaved_first(En, 1), unsaved_first(En, 3)],
            [
                "Save your files first: 1 file isn’t saved.",
                "Save your files first: 3 files aren’t saved."
            ]
        );
        assert_eq!(
            unsaved_first(Fr, 1),
            "Enregistre d'abord tes fichiers : 1 fichier n'est pas enregistré."
        );
        assert_eq!(
            restart_toast(En, "1.7.0"),
            [
                "Escouade is restarting to update",
                "Version 1.7.0 installs in 30 seconds.",
                "Later"
            ]
        );
    }

    #[tokio::test]
    async fn no_restart_with_files_left_unsaved() {
        let h = harness("upd-unsaved");
        let u = Updates::default();
        let log = ready(&u, Install::Ok).await;
        h.core.unsaved.store(2, Ordering::Release);
        let e = restart(&h.core, &u, Window::Front, |_| panic!("left")).unwrap_err();
        assert!(
            e.to_string().contains("2 fichiers ne sont pas enregistrés"),
            "{e}"
        );
        assert!(log.lock().is_empty());
        assert!(!h.core.quitting.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn an_install_that_fails_with_the_app_running_can_be_tried_again() {
        let h = harness("upd-fails");
        let u = Updates::default();
        ready(&u, Install::FailsBefore).await;
        assert!(install_ready(&h.core, &u, true, Window::Front, |_| panic!("left")).is_err());
        assert!(!h.core.quitting.load(Ordering::Acquire));
        // Not noted: the next start says nothing of it.
        assert!(!h.core.data.update_note_file().exists());
        assert!(install_ready(&h.core, &u, true, Window::Front, |_| panic!("left")).is_err());
    }

    fn write_note(data: &DataDir, note: serde_json::Value) {
        std::fs::write(data.update_note_file(), note.to_string()).unwrap();
    }

    #[test]
    fn the_next_start_knows_an_update_that_did_not_install() {
        let data = DataDir::new(crate::paths::test_dir("upd-note-failed"));
        write_note(
            &data,
            serde_json::json!({ "version": "1.6.0", "notes": "…", "window": "hidden" }),
        );
        // Still on 1.5.4: 1.6.0 did not install, and the window shows as at any start.
        let after = take_note(&data, "1.5.4");
        assert_eq!(after, Some(AfterUpdate::Failed("1.6.0".into())));
        assert_eq!(AfterUpdate::window(&after), Window::Front);
        assert!(!data.update_note_file().exists());
        assert_eq!(take_note(&data, "1.5.4"), None);
        let u = Updates::started(after);
        assert_eq!(u.failed().as_deref(), Some("1.6.0"));
        assert_eq!(u.take_installed(), None);
    }

    #[test]
    fn a_note_without_the_window_brings_it_in_front() {
        let data = DataDir::new(crate::paths::test_dir("upd-note-old"));
        write_note(
            &data,
            serde_json::json!({ "version": "1.6.0", "notes": "…" }),
        );
        let after = take_note(&data, "1.6.0");
        assert!(matches!(
            after,
            Some(AfterUpdate::Installed(_, Window::Front))
        ));
        assert_eq!(AfterUpdate::window(&None), Window::Front);
    }

    #[test]
    fn a_minimized_window_comes_back_minimized_after_a_restart_not_hidden_in_the_tray() {
        let noted = |w: Window| serde_json::to_value(w).unwrap();
        for macos in [false, true] {
            // On screen: in front or behind another app's window.
            assert_eq!(window_of(true, false, true, macos), Window::Front);
            assert_eq!(window_of(true, false, false, macos), Window::Behind);
            // Closed to the tray: it stays there.
            assert_eq!(window_of(false, false, false, macos), Window::Hidden);
            // Minimized: back in the taskbar or the Dock, with its button, not in the tray.
            assert_eq!(noted(window_of(true, true, false, macos)), "minimized");
            assert_eq!(noted(window_of(true, true, true, macos)), "minimized");
        }
        // macOS says a window minimized to the Dock is not visible: it is not in the tray for that.
        assert_eq!(noted(window_of(false, true, false, true)), "minimized");
        // Windows says it is: one that is not is in the tray, minimized or not.
        assert_eq!(window_of(false, true, false, false), Window::Hidden);
        // As the note of the install keeps it, read at the next start.
        let data = DataDir::new(crate::paths::test_dir("upd-note-minimized"));
        write_note(
            &data,
            serde_json::json!({ "version": "1.6.0", "notes": "…", "window": "minimized" }),
        );
        let after = take_note(&data, "1.6.0");
        assert!(after.is_some(), "the note is read");
        assert_eq!(noted(AfterUpdate::window(&after)), "minimized");
        // A note written before still reads as it did.
        write_note(
            &data,
            serde_json::json!({ "version": "1.6.0", "notes": "…", "window": "hidden" }),
        );
        assert_eq!(
            AfterUpdate::window(&take_note(&data, "1.6.0")),
            Window::Hidden
        );
    }

    #[test]
    fn the_app_bundle_is_found_from_its_executable() {
        assert_eq!(
            app_bundle(Path::new(
                "/Applications/Escouade.app/Contents/MacOS/escouade"
            )),
            Some(Path::new("/Applications/Escouade.app"))
        );
        // A build run from the repository has no bundle to replace.
        assert_eq!(
            app_bundle(Path::new("/code/escouade/src-tauri/target/debug/escouade")),
            None
        );
        assert_eq!(app_bundle(Path::new("/x/Contents/MacOS/escouade")), None);
    }

    #[tokio::test]
    async fn the_snapshot_counts_what_a_restart_would_cut() {
        let h = harness("upd-snapshot");
        let (p, _) = h.project(false).await;
        let ids: Vec<String> = {
            let mut ids = Vec::new();
            for _ in 0..5 {
                ids.push(h.core.create_agent(&p.id, None).await.unwrap().meta.id);
            }
            ids
        };
        let set = |i: usize, f: &dyn Fn(&mut crate::agent::AgentRt)| {
            let a = h.core.agents.read()[&ids[i]].clone();
            f(&mut a.lock());
        };
        set(0, &|rt| rt.meta.status = AgentStatus::Running);
        set(1, &|rt| rt.meta.status = AgentStatus::Waiting);
        set(2, &|rt| rt.setup = Some("npm ci (1/2)".into()));
        // Archived, it counts for nothing.
        set(3, &|rt| {
            rt.meta.status = AgentStatus::Running;
            rt.meta.archived = true;
        });
        h.core.tickets.write().push(crate::model::Ticket {
            project_id: p.id.clone(),
            step: Some("Validation…".into()),
            ..Default::default()
        });
        h.core.unsaved.store(3, Ordering::Release);
        *h.core.held.lock() = [ids[3].clone(), ids[4].clone()].into();
        let u = Updates::default();
        let working = h.core.working();
        u.set_presence(Presence {
            modal: true,
            testing: true,
            active_at: 1_000,
        });
        assert_eq!(
            snapshot(&h.core, &u, 61_000),
            Snapshot {
                turns: 2,
                questions: 1,
                setups: 1,
                validations: 1,
                works: 1,
                background: 0,
                launches: 0,
                // No terminal open.
                terminal_quiet: i64::MAX,
                testing: true,
                unsaved: 3,
                modal: true,
                inactive: 60_000,
                // No window in tests.
                visible: false,
            }
        );
        drop(working);
        assert_eq!(snapshot(&h.core, &u, 61_000).works, 0);
    }

    /// The rest of the app, for `snapshot`: no window, no one at the keyboard.
    fn background_of(h: &crate::core_tests::Harness) -> (usize, Option<Busy>) {
        let u = Updates::default();
        let s = snapshot(&h.core, &u, now_ms());
        (s.background, busy(&s))
    }

    #[tokio::test]
    async fn a_command_running_in_the_background_holds_off_a_restart_until_it_is_over() {
        use crate::plan::{RunStatus, SubAgent, WorkflowRun};
        let h = harness("upd-snapshot-background");
        let (p, _) = h.project(false).await;
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        assert_eq!(background_of(&h).0, 0);
        h.core
            .send_message(&id, "commande de fond 60s".into(), vec![])
            .await
            .unwrap();
        let row = h.bg_task(&id, "Ticker", false).await;
        // The turn is over, the command is not: it lives in the agent's process, and it is all
        // that keeps the app from being at rest.
        h.wait("only the command left", |h| {
            background_of(h) == (1, Some(Busy::Background))
        })
        .await;
        let turns = |h: &crate::core_tests::Harness| {
            h.items(&id).iter().filter(|i| i["kind"] == "turn").count()
        };
        let before = turns(&h);
        let agent = h.core.agent(&id).unwrap();

        // What runs counts one by one: a subagent and a workflow in the background too; what
        // is over, not.
        {
            let mut rt = agent.lock();
            let plan = rt.meta.plan.as_mut().unwrap();
            plan.agents.push(SubAgent::default());
            plan.agents.push(SubAgent {
                status: RunStatus::Done,
                ..Default::default()
            });
            plan.workflows.push(WorkflowRun::default());
            plan.workflows.push(WorkflowRun {
                status: RunStatus::Failed,
                ..Default::default()
            });
            plan.bg_tasks.push(crate::plan::BgTask {
                id: "ended".into(),
                status: RunStatus::Stopped,
                ..Default::default()
            });
        }
        assert_eq!(background_of(&h), (3, Some(Busy::Background)));
        {
            let mut rt = agent.lock();
            let plan = rt.meta.plan.as_mut().unwrap();
            plan.agents.clear();
            plan.workflows.clear();
            plan.bg_tasks.retain(|t| t.id != "ended");
        }
        assert_eq!(background_of(&h), (1, Some(Busy::Background)));

        // An archived agent counts for nothing, and one with no process neither: what ran in it
        // is gone with it.
        agent.lock().meta.archived = true;
        assert_eq!(background_of(&h), (0, None));
        agent.lock().meta.archived = false;
        assert_eq!(background_of(&h), (1, Some(Busy::Background)));
        let live = agent.lock().proc.take();
        assert_eq!(background_of(&h), (0, None));
        agent.lock().proc = live;
        assert_eq!(background_of(&h), (1, Some(Busy::Background)));

        // Stopped, it counts no more. Claude Code opens a turn to tell the agent: that is a turn,
        // not the command, and the app is at rest once it is over.
        h.core.stop_bg_task(&id, &row.id).await.unwrap();
        h.bg_task_status(&id, &row.id, RunStatus::Stopped).await;
        assert_eq!(background_of(&h).0, 0);
        h.wait("its turn over", |h| {
            turns(h) > before && !h.agent(&id).status.is_active()
        })
        .await;
        assert_eq!(background_of(&h), (0, None));
    }

    #[tokio::test]
    async fn a_message_being_delivered_holds_off_a_restart_until_its_turn_runs() {
        let h = harness("upd-send");
        let (p, _) = h.project(false).await;
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        let a = id.clone();
        // Its process starting, the agent not at work yet.
        let (stop, sampler) = h.sample_rest(move |c| {
            let running = c
                .agent(&a)
                .is_ok_and(|h| h.lock().meta.status == AgentStatus::Running);
            !running && c.works.load(Ordering::Acquire) > 0
        });
        within(
            "the message",
            h.core.send_message(&id, "Bonjour".into(), vec![]),
        )
        .await
        .unwrap();
        stop.store(true, Ordering::Release);
        let seen = sampler.join().unwrap();
        let sending: Vec<_> = seen.iter().filter(|(_, sending, _)| *sending).collect();
        assert!(!sending.is_empty(), "the message was never seen under way");
        assert!(sending.iter().all(|(_, _, then)| then.is_some()));
    }
}
