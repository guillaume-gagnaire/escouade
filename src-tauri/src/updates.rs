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
use crate::model::{now_ms, AgentStatus, Settings, UiEvent};
use crate::paths::DataDir;
use anyhow::{anyhow, bail, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

/// How long a check may take.
const TIMEOUT: Duration = Duration::from_secs(20);

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

type Fetching<'a> = Pin<Box<dyn Future<Output = Result<Box<dyn Package>>> + Send + 'a>>;

/// An update a check found: the plugin's, or a test's.
pub trait Release: Send + Sync {
    fn version(&self) -> &str;
    fn notes(&self) -> &str;
    /// Downloads it and checks its signature.
    fn fetch(&self) -> Fetching<'_>;
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

struct PluginRelease {
    update: Update,
    slot: Slot,
}

impl Release for PluginRelease {
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
            let bytes = update.download(|_, _| {}, || {}).await?;
            Ok(Box::new(PluginPackage {
                update,
                bytes,
                slot: self.slot.clone(),
            }) as Box<dyn Package>)
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
    /// Merges asked for by hand (« Merger »).
    pub merges: usize,
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
    Merge,
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
        (s.merges > 0, Busy::Merge),
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
    presence: Mutex<Presence>,
    plan: Mutex<Plan>,
    /// Merges asked for by hand under way.
    merges: AtomicUsize,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            found: Kept::default(),
            ready: Mutex::default(),
            downloads: AtomicU64::new(0),
            installing: AtomicBool::new(false),
            // Started, the window has just been seen.
            presence: Mutex::new(Presence {
                active_at: now_ms(),
                ..Presence::default()
            }),
            plan: Mutex::default(),
            merges: AtomicUsize::new(0),
        }
    }
}

/// A merge asked for by hand, under way until dropped.
pub struct Merging<'a>(&'a AtomicUsize);

impl Drop for Merging<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
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

    /// Downloads the update found as `id` and keeps it ready, unless it already is; an older one
    /// downloaded is no longer offered meanwhile. True when it is the one ready now.
    pub async fn download(&self, id: u32) -> Result<bool> {
        let release = self.found.take(id).ok_or_else(|| {
            anyhow!("Cette mise à jour n'est plus proposée : recherche-la de nouveau.")
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
        let package = release.fetch().await?;
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

    /// A merge asked for by hand, under way while the guard lives.
    pub fn merging(&self) -> Merging<'_> {
        self.merges.fetch_add(1, Ordering::AcqRel);
        Merging(&self.merges)
    }
}

/// Looks for a newer release through the proxy, its certificate checked unless TLS verification
/// is off; the update found is kept for its download. None found: the one downloaded, if any,
/// was withdrawn.
pub async fn check<R: Runtime>(
    app: &AppHandle<R>,
    settings: &Settings,
    updates: &Updates,
) -> Result<Option<Found>> {
    let slot = Slot::default();
    let mut builder = app.updater_builder().timeout(TIMEOUT).on_before_exit({
        let (slot, app) = (slot.clone(), app.clone());
        move || {
            // Taken first: the lock is not held while the app stops.
            let hook = slot.lock().take();
            if let Some(hook) = hook {
                hook();
            }
            // What the plugin's own hook does, which this one replaces: the tray icon goes now.
            app.cleanup_before_exit();
        }
    });
    let proxy = settings.proxy_url.trim();
    if !proxy.is_empty() {
        builder = builder.proxy(
            proxy
                .parse()
                .map_err(|e| anyhow!("adresse du proxy invalide : {e}"))?,
        );
    }
    if settings.insecure_tls {
        builder = builder.configure_client(|c| c.danger_accept_invalid_certs(true));
    }
    let Some(update) = builder.build()?.check().await? else {
        updates.withdraw();
        return Ok(None);
    };
    Ok(Some(
        updates.offer(Arc::new(PluginRelease { update, slot })),
    ))
}

/// What the app is doing now that a restart would cut.
pub fn snapshot<R: Runtime>(core: &Core<R>, updates: &Updates, now: i64) -> Snapshot {
    let mut s = Snapshot::default();
    for h in core.agents.read().values() {
        let rt = h.lock();
        if rt.meta.archived {
            continue;
        }
        match rt.meta.status {
            AgentStatus::Running => s.turns += 1,
            AgentStatus::Waiting => s.questions += 1,
            _ => {}
        }
        if rt.setup.is_some() {
            s.setups += 1;
        }
    }
    s.validations = core
        .tickets
        .read()
        .iter()
        .filter(|t| t.step.is_some())
        .count();
    s.merges = updates.merges.load(Ordering::Acquire);
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

/// Installs the update ready, the app stopped cleanly first (`Core::shutdown`: its agents and
/// terminals stop, its state is saved) and the update noted for the next start
/// (`take_installed`). With `relaunch`, the app starts again on it, else it stays closed:
/// `leave` restarts or closes it once the update is in place (`apply`). False when there is
/// nothing to install, or an install is under way already; an error when it failed with the app
/// still running.
pub fn install_ready<R: Runtime>(
    core: &Arc<Core<R>>,
    updates: &Updates,
    relaunch: bool,
    leave: impl FnOnce(bool),
) -> Result<bool> {
    let Some(ready) = updates.ready() else {
        return Ok(false);
    };
    if updates.installing.swap(true, Ordering::AcqRel) {
        return Ok(false);
    }
    let note = core.data.update_note_file();
    let noted = serde_json::to_vec(&Installed {
        version: ready.version.clone(),
        notes: ready.notes.clone(),
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
    let installed = apply(&*ready.package, relaunch, stop, STOP_LIMIT, leave);
    if installed.is_err() {
        // The app still runs: the next try, or closing it, installs it again.
        updates.installing.store(false, Ordering::Release);
        let _ = std::fs::remove_file(&note);
    }
    installed.map(|()| true)
}

/// « Redémarrer maintenant », or the automatic restart: the update ready installs and the app
/// starts again on it. Never while the editor holds unsaved files.
pub fn restart<R: Runtime>(
    core: &Arc<Core<R>>,
    updates: &Updates,
    leave: impl FnOnce(bool),
) -> Result<()> {
    match core.unsaved.load(Ordering::Acquire) {
        0 => {}
        1 => bail!("Enregistre d'abord tes fichiers : 1 fichier n'est pas enregistré."),
        n => bail!("Enregistre d'abord tes fichiers : {n} fichiers ne sont pas enregistrés."),
    }
    if !install_ready(core, updates, true, leave)? {
        bail!("Aucune mise à jour n'est prête : recherche-la de nouveau.");
    }
    Ok(())
}

/// The note of the update the app stopped for (`install_ready`), taken once: when the app now
/// runs its version (an install that failed leaves the note of a version it does not run).
pub fn take_installed(data: &DataDir, version: &str) -> Option<Installed> {
    let file = data.update_note_file();
    let bytes = std::fs::read(&file).ok()?;
    let _ = std::fs::remove_file(&file);
    let note: Installed = serde_json::from_slice(&bytes).ok()?;
    (note.version == version).then_some(note)
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

/// « Redémarrer maintenant », or the automatic restart, off the async runtime (the stop and the
/// install block).
pub async fn restart_now<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let core = app.state::<Arc<Core<R>>>().inner().clone();
        let env = app.env();
        restart(&core, &app.state::<Updates>(), |relaunch| {
            leave(&app, &env, relaunch)
        })
    })
    .await?
}

/// « Quitter »: the app stops; with an update ready, it installs silently and the app stays
/// closed.
pub fn quit<R: Runtime>(app: &AppHandle<R>) {
    let core = app.state::<Arc<Core<R>>>().inner().clone();
    if app.state::<Updates>().ready().is_none() {
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
        let installed = install_ready(&core, &app.state::<Updates>(), false, |relaunch| {
            leave(&app, &env, relaunch)
        });
        if !matches!(installed, Ok(true)) {
            if let Err(e) = installed {
                log::error!("update not installed when closing: {e:#}");
            }
            // The app closes all the same.
            core.shutdown();
            app.exit(0);
        }
    });
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
    let on = core.settings.read().auto_update
        && ready.is_some()
        && !updates.installing.load(Ordering::Acquire);
    let busy = if on {
        busy(&snapshot(&core, &updates, now))
    } else {
        None
    };
    match updates.advance(on, busy, now) {
        Step::Wait => {}
        Step::Warn(at) => {
            core.hub.emit(UiEvent::UpdateRestart { at: Some(at) });
            if core.settings.read().os_notifications {
                let version = ready.map(|r| r.version.clone()).unwrap_or_default();
                let a = app.clone();
                crate::notify::toast_with_button(
                    app,
                    "Escouade redémarre pour se mettre à jour",
                    &format!("La version {version} s'installe dans 30 secondes."),
                    "Plus tard",
                    move |later| {
                        if later {
                            postpone(&a);
                        } else {
                            crate::notify::show_main(&a);
                        }
                    },
                );
            }
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
    use std::sync::atomic::AtomicBool;
    use tokio::sync::Notify;

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

    /// The app at rest: nothing under way, the user away for 5 minutes, window on screen.
    fn rest() -> Snapshot {
        Snapshot {
            inactive: AWAY,
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
            (with(|s| s.merges = 1), Busy::Merge),
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
        // Used a minute ago, the window hidden since: at rest.
        let hidden = Snapshot {
            inactive: 60_000,
            visible: false,
            ..Snapshot::default()
        };
        assert_eq!(busy(&hidden), None);
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

    // ---------- download ----------

    /// What a fake package's install saw and did, in order.
    type Log = Arc<Mutex<Vec<String>>>;

    struct FakeRelease {
        version: String,
        fetches: AtomicUsize,
        /// The download fails.
        broken: AtomicBool,
        /// The download waits for it.
        gate: Option<Arc<Notify>>,
        log: Log,
        /// What the package's install does: like the plugin's, or failing before or after
        /// `before_exit`.
        install: Install,
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
            gate: None,
            log: Log::default(),
            install: Install::Ok,
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
                if self.broken.load(Ordering::Acquire) {
                    bail!("Download request failed with status: 502");
                }
                Ok(Box::new(FakePackage {
                    log: self.log.clone(),
                    install: self.install,
                }) as Box<dyn Package>)
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
        while newer.fetches.load(Ordering::Acquire) == 0 {
            tokio::task::yield_now().await;
        }
        assert!(u.ready().is_none());
        gate.notify_one();
        assert!(downloading.await.unwrap().unwrap());
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
        while r.fetches.load(Ordering::Acquire) == 0 {
            tokio::task::yield_now().await;
        }
        u.withdraw();
        gate.notify_one();
        assert!(!downloading.await.unwrap().unwrap());
        assert!(u.ready().is_none());
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
        restart(&core, &u, move |relaunch| {
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
        assert_eq!(
            take_installed(&core.data, "1.6.0"),
            Some(Installed {
                version: "1.6.0".into(),
                notes: "- Mises à jour silencieuses".into()
            })
        );
        // Told once.
        assert_eq!(take_installed(&core.data, "1.6.0"), None);
    }

    #[tokio::test]
    async fn closing_the_app_installs_the_update_without_starting_it_again() {
        let h = harness("upd-quit");
        let u = Updates::default();
        let log = ready(&u, Install::Ok).await;
        let left = log.clone();
        assert!(install_ready(&h.core, &u, false, move |relaunch| {
            left.lock().push(format!("leave (relaunch: {relaunch})"))
        })
        .unwrap());
        assert!(h.core.quitting.load(Ordering::Acquire));
        assert_eq!(
            *log.lock(),
            ["installer (relaunch: false)", "leave (relaunch: false)"]
        );
        // One install at a time.
        assert!(!install_ready(&h.core, &u, false, |_| {}).unwrap());
    }

    #[tokio::test]
    async fn nothing_installs_without_an_update_downloaded() {
        let h = harness("upd-none");
        let u = Updates::default();
        assert!(!install_ready(&h.core, &u, true, |_| panic!("left")).unwrap());
        assert!(restart(&h.core, &u, |_| panic!("left")).is_err());
        assert!(!h.core.quitting.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn no_restart_with_files_left_unsaved() {
        let h = harness("upd-unsaved");
        let u = Updates::default();
        let log = ready(&u, Install::Ok).await;
        h.core.unsaved.store(2, Ordering::Release);
        let e = restart(&h.core, &u, |_| panic!("left")).unwrap_err();
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
        assert!(install_ready(&h.core, &u, true, |_| panic!("left")).is_err());
        assert!(!h.core.quitting.load(Ordering::Acquire));
        // Not noted: the next start says nothing of it.
        assert!(!h.core.data.update_note_file().exists());
        assert!(install_ready(&h.core, &u, true, |_| panic!("left")).is_err());
    }

    #[test]
    fn the_note_of_a_version_the_app_does_not_run_is_dropped() {
        let dir = crate::paths::test_dir("upd-note");
        let data = DataDir::new(dir);
        let note = Installed {
            version: "1.6.0".into(),
            notes: "…".into(),
        };
        std::fs::write(data.update_note_file(), serde_json::to_vec(&note).unwrap()).unwrap();
        assert_eq!(take_installed(&data, "1.5.4"), None);
        assert!(!data.update_note_file().exists());
        assert_eq!(take_installed(&data, "1.5.4"), None);
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
        let u = Updates::default();
        let _merging = u.merging();
        u.set_presence(Presence {
            modal: true,
            testing: true,
            active_at: 1_000,
        });
        assert_eq!(
            snapshot(&h.core, &u, 61_000),
            Snapshot {
                turns: 1,
                questions: 1,
                setups: 1,
                validations: 1,
                merges: 1,
                testing: true,
                unsaved: 3,
                modal: true,
                inactive: 60_000,
                // No window in tests.
                visible: false,
            }
        );
        drop(_merging);
        assert_eq!(snapshot(&h.core, &u, 61_000).merges, 0);
    }
}
