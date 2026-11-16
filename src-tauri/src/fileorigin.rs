//! Where a file the page hands the window lies on the disk (ESC-95, « chemin d'origine »).
//!
//! A `File` the WebView gives the page has no path, and Tauri's own drag and drop, which would give
//! it, is off (it would break the HTML drag and drop of the app). WebView2 has a way of its own: the
//! page sends its host a message with `File` objects (`chrome.webview.postMessageWithAdditionalObjects`)
//! and the host reads their paths (`ICoreWebView2File`). The page sends `{ "escouadeFileOrigin": id }`
//! with one file, then asks `file_origin(id)` for what the host read. Under Linux and macOS there is
//! no such bridge here: the command answers nothing at once, there is no path of origin, and the
//! page copies the file. A path the page would read from the drop's own data (`text/uri-list`) is
//! the page's word and no one else's: it is not one the host read, so it is never trusted.
//!
//! Only a path the host itself read may become a file joined by its path, since Escouade lets the
//! agent read such a file without asking (`AgentMeta::granted_files`): the page never gives a path
//! to `attach_origin`, only the id `file_origin` answered for. The command keeps what it answers
//! (`handed`) for that one taking (`take_handed`); an id the host read a path for and the page did
//! not ask the path of is of no use.
//!
//! The message goes to wry's own handler too, which hands it to Tauri's IPC. Tauri does not know it
//! for an invoke (`handle_ipc_message` does not find `cmd` in it) and drops it, with a
//! `console.error` in the page as its only reaction: nothing is run and nothing is answered.

use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Manager, Runtime, WebviewWindow};
use tokio::sync::Notify;
use tokio::time::{timeout_at, Instant};

/// How many paths wait to be asked for, and how many that were asked for wait to be joined by: the
/// oldest leave first, in each table. A path is asked for, then joined by, within moments of its
/// arrival, so only a page that never goes on (a file dropped, its drop gone) leaves some.
const MAX_ENTRIES: usize = 256;

/// The longest an id may be (a UUID is 36).
const MAX_ID: usize = 64;

/// How long `file_origin` waits for a path: the message of the page crosses the WebView's process
/// to the host's thread, and the command may be asked first. A `File` that is no file of the disk
/// (a screenshot pasted) makes WebView2 refuse the message in the page itself (a TypeError, which
/// `file-origin.ts` turns into « no path » at once): this delay is for a message that got lost.
const WAIT: Duration = Duration::from_secs(2);

/// The paths the window sent, by the id the page gave each: managed by the app, and held by the
/// core (`Core::file_origins`), which takes the ones that were handed out.
#[derive(Clone, Default)]
pub struct FileOrigins(Arc<Shared>);

#[derive(Default)]
struct Shared {
    /// What the host read, not asked for yet. Oldest first.
    paths: Mutex<VecDeque<(String, String)>>,
    /// What `file_origin` answered, kept for the one file that is joined by it. Oldest first.
    handed: Mutex<VecDeque<(String, String)>>,
    /// Woken when a path arrives.
    arrived: Notify,
}

/// `path` for `id`, the newest of `table`, which holds `MAX_ENTRIES` at most: a second one for the
/// same id replaces the first.
fn put(table: &mut VecDeque<(String, String)>, id: String, path: String) {
    table.retain(|(known, _)| *known != id);
    table.push_back((id, path));
    while table.len() > MAX_ENTRIES {
        table.pop_front();
    }
}

/// The path of `id` in `table`, which forgets it.
fn pull(table: &mut VecDeque<(String, String)>, id: &str) -> Option<String> {
    let at = table.iter().position(|(known, _)| known == id)?;
    table.remove(at).map(|(_, path)| path)
}

impl FileOrigins {
    /// Keeps `path` for `id` (a second one for the same id replaces the first) and wakes whoever waits.
    /// Only the WebView2 listener (Windows) calls it; the tests call it everywhere.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn deposit(&self, id: String, path: String) {
        put(&mut self.0.paths.lock(), id, path);
        self.0.arrived.notify_waiters();
    }

    /// The path for `id`, if it arrived: it is forgotten as it is given.
    pub(crate) fn take(&self, id: &str) -> Option<String> {
        pull(&mut self.0.paths.lock(), id)
    }

    /// `take`, and the path is kept for `take_handed`: it is the page that was told it.
    pub(crate) fn hand(&self, id: &str) -> Option<String> {
        let path = self.take(id)?;
        put(&mut self.0.handed.lock(), id.to_string(), path.clone());
        Some(path)
    }

    /// The path `file_origin` answered for `id`, forgotten as it is given: a file is joined by it
    /// once. None for an id that was never answered (a path only the host read, or none at all).
    pub(crate) fn take_handed(&self, id: &str) -> Option<String> {
        pull(&mut self.0.handed.lock(), id)
    }

    /// The path for `id` (`hand`): the one already there, or the first to arrive within `delay`.
    pub(crate) async fn wait(&self, id: &str, delay: Duration) -> Option<String> {
        let deadline = Instant::now() + delay;
        loop {
            // Made before the look: a path that arrives between the two wakes it all the same.
            let arrived = self.0.arrived.notified();
            if let Some(path) = self.hand(id) {
                return Some(path);
            }
            if timeout_at(deadline, arrived).await.is_err() {
                // A path may have come as the delay ended.
                return self.hand(id);
            }
        }
    }
}

/// Whether `id` is one the page may give: 1 to 64 letters, digits, `-` or `_`.
fn valid_id(id: &str) -> bool {
    (1..=MAX_ID).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The id a page message asks for the path of a file with: `{ "escouadeFileOrigin": "<id>" }`.
/// Any other message (Tauri's own, another key, a bad id, no JSON) is none of ours.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn parse_message(message: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Asked {
        #[serde(rename = "escouadeFileOrigin")]
        id: String,
    }
    let Asked { id } = serde_json::from_str(message).ok()?;
    valid_id(&id).then_some(id)
}

/// The path of the file on the disk that the page sent as `id`, as the host read it: nothing
/// where the system gives none, or if it did not arrive in time. What it answers is kept for
/// `attach_origin`, which takes the id (`take_handed`) and never a path the page makes up.
#[escouade_command::command]
pub async fn file_origin<R: Runtime>(app: tauri::AppHandle<R>, id: String) -> Option<String> {
    if !cfg!(windows) || !valid_id(&id) {
        return None;
    }
    let origins = app.try_state::<FileOrigins>()?.inner().clone();
    origins.wait(&id, WAIT).await
}

/// Puts the listener on the WebView of `window`, which it keeps for good: the app only ever makes
/// its main window once (a reload keeps the WebView, and with it the listener).
#[cfg(windows)]
pub fn install<R: Runtime>(window: &WebviewWindow<R>, origins: &FileOrigins) {
    let origins = origins.clone();
    // The WebView's COM objects are only for its own thread: `with_webview` runs this on it.
    let asked = window.with_webview(move |webview| {
        // SAFETY: `controller` is the live controller of this WebView, used on its thread.
        if let Err(e) = unsafe { windows_side::listen(&webview.controller(), origins) } {
            log::error!("the paths of dropped files are not read: {e}");
        }
    });
    if let Err(e) = asked {
        log::error!("the paths of dropped files are not read: {e}");
    }
}

/// No bridge on the other systems: the command answers nothing.
#[cfg(not(windows))]
pub fn install<R: Runtime>(_window: &WebviewWindow<R>, _origins: &FileOrigins) {}

#[cfg(windows)]
mod windows_side {
    use super::{parse_message, FileOrigins};
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Controller, ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs,
        ICoreWebView2WebMessageReceivedEventArgs2,
    };
    use webview2_com::{take_pwstr, WebMessageReceivedEventHandler};
    use windows_core::{Interface, PWSTR};

    /// Adds to the WebView a handler of the messages the page sends it. wry has its own, for
    /// Tauri's IPC: this one comes with it, and does not take its place.
    ///
    /// # Safety
    /// `controller` must be the live controller of the WebView, and this called on its thread.
    pub unsafe fn listen(
        controller: &ICoreWebView2Controller,
        origins: FileOrigins,
    ) -> windows_core::Result<()> {
        let webview = controller.CoreWebView2()?;
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            if let Some((id, path)) = args.as_ref().and_then(origin_of) {
                origins.deposit(id, path);
            }
            Ok(())
        }));
        // The registration lasts as long as the WebView: it is never removed.
        let mut token = 0;
        webview.add_WebMessageReceived(&handler, &mut token)
    }

    /// The id the page gave a file with the message, and the file's path, when the message is
    /// ours and has a file of the disk. Called for every message of the page, Tauri's IPC
    /// included, which has no object with it: `AdditionalObjects` tells them apart before
    /// anything is read.
    fn origin_of(args: &ICoreWebView2WebMessageReceivedEventArgs) -> Option<(String, String)> {
        // SAFETY: `args` is the event's, alive for the call. The strings the calls give are
        // allocated by COM (`CoTaskMemAlloc`) and taken, and so freed, by `take_pwstr`.
        unsafe {
            let objects = args
                .cast::<ICoreWebView2WebMessageReceivedEventArgs2>()
                .ok()?
                .AdditionalObjects()
                .ok()?;
            let mut count = 0;
            objects.Count(&mut count).ok()?;
            if count == 0 {
                return None;
            }
            let mut message = PWSTR::null();
            args.TryGetWebMessageAsString(&mut message).ok()?;
            let id = parse_message(&take_pwstr(message))?;
            let file = objects
                .GetValueAtIndex(0)
                .ok()?
                .cast::<ICoreWebView2File>()
                .ok()?;
            let mut path = PWSTR::null();
            file.Path(&mut path).ok()?;
            Some((id, take_pwstr(path)))
        }
    }
}
