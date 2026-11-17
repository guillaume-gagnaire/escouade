//! What the core asks of the process it runs in (ESC-451, `docs/REMOTE.md`, « La fondation
//! commune »): the app's version and resources, the notifications and the tray, what opens outside
//! Escouade, the clipboard, the devices to let go of at the end.
//!
//! The core holds a `Host` in place of Tauri's `AppHandle`, so that it runs without a window:
//! - `TauriHost`, the desktop app's, does all of it as before, through Tauri and its plugins;
//! - `HeadlessHost` has no window: what the desktop shows (a notification, the taskbar's flash, the
//!   tray's count, a file to open) it only publishes, as a `HostEvent`, for whoever subscribes
//!   (the tests, then `escouade-server` and its clients).
//!
//! The events of the interface (`UiEvent`) do not go through the host: the core sends them to its
//! `Hub`, whatever runs it.

use crate::fileorigin::FileOrigins;
use crate::i18n::Lang;
use parking_lot::Mutex;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
#[cfg(feature = "desktop")]
use tauri::{AppHandle, Manager, Runtime};

/// A future of the host's, which the core awaits from any task.
pub type HostFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What is opened outside Escouade, on the machine of the host.
/// Sent to the clients of the remote API as `{"kind": "url", "target": …}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "target", rename_all = "camelCase")]
pub enum Opened {
    /// A link, in the browser.
    Url(String),
    /// A file in its program, or a folder in the file manager.
    Path(PathBuf),
    /// A file shown, selected, in its folder.
    Reveal(PathBuf),
}

/// What the core asks of its host. Each method stands for what the desktop does: a host without a
/// window does what it can of it (`HeadlessHost`: publishes it).
pub trait Host: Send + Sync + 'static {
    /// The app's version (`1.2.3`).
    fn version(&self) -> String;
    /// The folder of the resources the installer brought (the seed of the plugins, ESC-338).
    fn resource_dir(&self) -> Option<PathBuf>;
    /// The table of the dropped files' origins the window's listener fills (Windows).
    fn file_origins(&self) -> FileOrigins;
    /// The user looks at the app: its window shown, focused and not minimized.
    fn attended(&self) -> bool;
    /// The app's window is on screen: shown and not minimized.
    fn window_visible(&self) -> bool;
    /// The chime of a notification.
    fn chime(&self);
    /// The window draws the eye (the taskbar flashes, the Dock bounces).
    fn flash(&self);
    /// A system notification. `on_click` runs when the user activates it, once the host has brought
    /// its window forward.
    fn notify(&self, title: &str, body: &str, on_click: Box<dyn Fn() + Send + 'static>);
    /// How many agents wait for the user: the tray's icon and tooltip.
    fn show_waiting(&self, waiting: usize, lang: Lang);
    /// The native menus (the tray's, macOS' app menu) written again in `lang`.
    fn relabel(&self, lang: Lang, waiting: usize);
    /// `what` opened outside Escouade. Blocking: called off the async runtime's threads.
    fn open(&self, what: Opened) -> anyhow::Result<()>;
    /// The image on the clipboard, as PNG; empty without one.
    fn clipboard_image(&self) -> HostFuture<'_, anyhow::Result<Vec<u8>>>;
    /// The devices the app holds (the microphone) let go of, as the core stops.
    fn release_devices(&self);
    /// What it publishes from now on, for a client of the remote API to render (ESC-478): a host without a window's
    /// (`HeadlessHost`); none for the desktop's, which shows it itself.
    fn events(&self) -> Option<tokio::sync::mpsc::UnboundedReceiver<HostEvent>> {
        None
    }
    /// Where the core keeps its secrets, when the host chooses (`escouade-server`: a file readable by its owner alone,
    /// ESC-514); none: the system's keychain, as the desktop does (`integrations::secrets::store_for`).
    fn secrets(&self) -> Option<Arc<dyn crate::integrations::secrets::SecretStore>> {
        None
    }
    /// Whether the secrets are in a file of the data rather than in a keychain: the client says so (`welcome`).
    fn secrets_in_file(&self) -> bool {
        false
    }
}

/// What `Core::load` takes for its host: a Tauri app's handle (the desktop's and most tests'), or a
/// host of its own.
pub trait IntoHost {
    fn into_host(self) -> Arc<dyn Host>;
}

#[cfg(feature = "desktop")]
impl<R: Runtime> IntoHost for AppHandle<R> {
    fn into_host(self) -> Arc<dyn Host> {
        Arc::new(TauriHost::new(self))
    }
}

impl<H: Host> IntoHost for Arc<H> {
    fn into_host(self) -> Arc<dyn Host> {
        self
    }
}

impl IntoHost for Arc<dyn Host> {
    fn into_host(self) -> Arc<dyn Host> {
        self
    }
}

// ---------- the desktop's ----------

/// The desktop app's host: Tauri's window, tray, notifications, opener and clipboard.
#[cfg(feature = "desktop")]
pub struct TauriHost<R: Runtime> {
    app: AppHandle<R>,
}

#[cfg(feature = "desktop")]
impl<R: Runtime> TauriHost<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

#[cfg(feature = "desktop")]
impl<R: Runtime> Host for TauriHost<R> {
    fn version(&self) -> String {
        self.app.package_info().version.to_string()
    }

    fn resource_dir(&self) -> Option<PathBuf> {
        self.app.path().resource_dir().ok()
    }

    fn file_origins(&self) -> FileOrigins {
        // The app makes it before the core, when it has a window.
        self.app
            .try_state::<FileOrigins>()
            .map(|s| s.inner().clone())
            .unwrap_or_default()
    }

    fn attended(&self) -> bool {
        crate::notify::window_attended(&self.app)
    }

    fn window_visible(&self) -> bool {
        self.app
            .get_webview_window("main")
            .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
    }

    fn chime(&self) {
        crate::notify::play_chime();
    }

    fn flash(&self) {
        crate::notify::flash(&self.app);
    }

    fn notify(&self, title: &str, body: &str, on_click: Box<dyn Fn() + Send + 'static>) {
        let app = self.app.clone();
        crate::notify::toast(&self.app, title, body, move || {
            crate::notify::show_main(&app);
            on_click();
        });
    }

    fn show_waiting(&self, waiting: usize, lang: Lang) {
        if let Some(tray) = self.app.tray_by_id(crate::menus::TRAY_ID) {
            let _ = tray.set_icon(crate::notify::tray_icon(&self.app, waiting));
            let _ = tray.set_tooltip(Some(crate::menus::tray_tooltip(lang, waiting)));
        }
    }

    fn relabel(&self, lang: Lang, waiting: usize) {
        crate::menus::relabel(&self.app, lang, waiting);
    }

    fn open(&self, what: Opened) -> anyhow::Result<()> {
        use crate::outside::{open, What};
        match &what {
            Opened::Url(url) => open(What::Url(url)),
            Opened::Path(path) => open(What::Path(path)),
            Opened::Reveal(path) => {
                crate::fsedit::reveal(crate::fsedit::Reveal::Select(path.clone()))
            }
        }
    }

    fn clipboard_image(&self) -> HostFuture<'_, anyhow::Result<Vec<u8>>> {
        Box::pin(crate::clipboard::image_png(&self.app))
    }

    fn release_devices(&self) {
        if let Some(v) = self.app.try_state::<Arc<crate::voice::Voice>>() {
            v.shutdown();
        }
    }
}

// ---------- without a window ----------

/// What a host without a window was asked to show or open, published for its subscribers: the
/// client renders it on its own machine (`docs/REMOTE.md`, « Ce qui change de machine »).
/// Sent to the clients of the remote API as `{"type": "notify", "title": …}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum HostEvent {
    Chime,
    Flash,
    Notify { title: String, body: String },
    Waiting { count: usize },
    Open(Opened),
}

/// The host of a process without a window (the tests, then `escouade-server`): it publishes what
/// the desktop would show, and has neither a window nor a tray, a clipboard or devices.
pub struct HeadlessHost {
    version: String,
    resources: Option<PathBuf>,
    origins: FileOrigins,
    subscribers: Mutex<Vec<tokio::sync::mpsc::UnboundedSender<HostEvent>>>,
    /// The secrets' store it gives the core (`escouade-server`'s file), if any.
    secrets: Option<Arc<dyn crate::integrations::secrets::SecretStore>>,
}

impl Default for HeadlessHost {
    fn default() -> Self {
        Self::new(None)
    }
}

impl HeadlessHost {
    /// `resources`: the folder of the resources beside the program, if it has one.
    pub fn new(resources: Option<PathBuf>) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            resources,
            origins: FileOrigins::default(),
            subscribers: Mutex::new(Vec::new()),
            secrets: None,
        }
    }

    /// The core's secrets kept in a file (`escouade-server`, ESC-514: no keychain on a server).
    pub fn with_secrets(
        mut self,
        store: Arc<dyn crate::integrations::secrets::SecretStore>,
    ) -> Self {
        self.secrets = Some(store);
        self
    }

    /// The events from now on. A subscriber that went is forgotten at the next event.
    pub fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<HostEvent> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.subscribers.lock().push(tx);
        rx
    }

    fn publish(&self, event: HostEvent) {
        self.subscribers
            .lock()
            .retain(|tx| tx.send(event.clone()).is_ok());
    }
}

impl Host for HeadlessHost {
    fn version(&self) -> String {
        self.version.clone()
    }

    fn resource_dir(&self) -> Option<PathBuf> {
        self.resources.clone()
    }

    fn file_origins(&self) -> FileOrigins {
        self.origins.clone()
    }

    /// No one looks at a window there is not: every notification goes out.
    fn attended(&self) -> bool {
        false
    }

    fn window_visible(&self) -> bool {
        false
    }

    fn chime(&self) {
        self.publish(HostEvent::Chime);
    }

    fn flash(&self) {
        self.publish(HostEvent::Flash);
    }

    fn notify(&self, title: &str, body: &str, _on_click: Box<dyn Fn() + Send + 'static>) {
        self.publish(HostEvent::Notify {
            title: title.to_string(),
            body: body.to_string(),
        });
    }

    fn show_waiting(&self, waiting: usize, _lang: Lang) {
        self.publish(HostEvent::Waiting { count: waiting });
    }

    fn relabel(&self, _lang: Lang, _waiting: usize) {}

    fn open(&self, what: Opened) -> anyhow::Result<()> {
        self.publish(HostEvent::Open(what));
        Ok(())
    }

    fn clipboard_image(&self) -> HostFuture<'_, anyhow::Result<Vec<u8>>> {
        Box::pin(async {
            anyhow::bail!(tr!(
                "pas de presse-papiers sans bureau",
                "no clipboard without a desktop"
            ))
        })
    }

    fn release_devices(&self) {}

    fn events(&self) -> Option<tokio::sync::mpsc::UnboundedReceiver<HostEvent>> {
        Some(self.subscribe())
    }

    fn secrets(&self) -> Option<Arc<dyn crate::integrations::secrets::SecretStore>> {
        self.secrets.clone()
    }

    fn secrets_in_file(&self) -> bool {
        self.secrets.is_some()
    }
}
