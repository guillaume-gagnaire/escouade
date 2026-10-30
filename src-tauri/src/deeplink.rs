//! The deep link `escouade://plugin/<id>` (STORE-11, ESC-287): the store's « Ouvrir dans Escouade ». The system hands it to
//! the app (its argv on Windows and Linux, an Apple event on macOS); started already, the second launch hands it over to the
//! first (`single-instance`). It opens the plugin's page in the view Plugins and never installs anything: installing stays the
//! user's gesture, with its permissions shown. A link the app cannot read is said, and nothing else is done.

use crate::core::Core;
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};

/// The scheme the app registers (`plugins.deep-link.desktop.schemes` of tauri.conf.json).
pub const SCHEME: &str = "escouade";

/// The longest link read: an id is 100 characters at most.
const MAX_LEN: usize = 300;

/// What a link asks, for the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DeepLink {
    /// The page of the plugin `id`: its page in the store, its own when it is installed.
    Plugin { id: String },
    /// A link of the scheme the app cannot read: said, nothing done.
    Bad { url: String },
}

/// What `url` asks. `escouade://plugin/<id>`, the id a plugin's (a reversed domain name in lower case), a `/` after it
/// allowed; anything else (another path, a query, a fragment, an id that is no plugin's) is a bad link.
pub fn parse(url: &str) -> DeepLink {
    let bad = || DeepLink::Bad { url: shown(url) };
    if url.len() > MAX_LEN {
        return bad();
    }
    let Some(rest) = strip_scheme(url) else {
        return bad();
    };
    let Some(rest) = strip_prefix_ci(rest, "plugin/") else {
        return bad();
    };
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let Some(id) = percent_decode(rest) else {
        return bad();
    };
    if crate::plugins::manifest::is_id(&id) {
        DeepLink::Plugin { id }
    } else {
        bad()
    }
}

/// The links of the scheme among a launch's arguments (the program's name first, skipped): Windows and Linux give the link
/// as an argument (`"escouade.exe" "%1"`, `Exec=escouade %u`), to the first launch and, through `single-instance`, from a
/// second one.
pub fn links_in<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .skip(1)
        .map(|a| a.as_ref().to_string())
        .filter(|a| strip_scheme_name(a).is_some())
        .collect()
}

/// The links not yet taken by the window: one that comes before the window listens waits here.
#[derive(Default)]
pub struct Inbox(Mutex<Vec<DeepLink>>);

impl Inbox {
    pub fn put(&self, link: DeepLink) {
        self.0.lock().push(link);
    }

    pub fn take(&self) -> Vec<DeepLink> {
        std::mem::take(&mut *self.0.lock())
    }
}

/// Reads the links, puts them in the inbox, brings the window forward and tells it to take them.
pub fn deliver<R: Runtime>(app: &AppHandle<R>, urls: impl IntoIterator<Item = String>) {
    let mut any = false;
    for url in urls {
        let link = parse(&url);
        match &link {
            DeepLink::Plugin { id } => log::info!("deep link: the page of {id}"),
            DeepLink::Bad { url } => log::warn!("deep link not understood: {url}"),
        }
        if let Some(inbox) = app.try_state::<Inbox>() {
            inbox.put(link);
            any = true;
        }
    }
    if !any {
        return;
    }
    crate::notify::show_main(app);
    if let Some(core) = app.try_state::<Arc<Core>>() {
        core.hub.emit(crate::model::UiEvent::DeepLink {});
    }
}

/// What follows `escouade:` (with `//` or not, any case), when `url` is of the scheme.
fn strip_scheme(url: &str) -> Option<&str> {
    let rest = strip_scheme_name(url)?;
    Some(rest.strip_prefix("//").unwrap_or(rest))
}

fn strip_scheme_name(url: &str) -> Option<&str> {
    strip_prefix_ci(url.trim(), SCHEME)?.strip_prefix(':')
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

/// `%2E` and the like decoded; None for a `%` not followed by two hex digits, or bytes that are not UTF-8.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The link as the window shows it: cut short, without control characters.
fn shown(url: &str) -> String {
    let clean: String = url.trim().chars().filter(|c| !c.is_control()).collect();
    if clean.chars().count() > 120 {
        format!("{}…", clean.chars().take(120).collect::<String>())
    } else {
        clean
    }
}

/// At the start: the link the app was launched with (Windows, Linux), the links macOS hands over from now on, and on Linux
/// an AppImage registered as the scheme's handler (no installer did it).
pub fn at_start<R: Runtime>(app: &AppHandle<R>) {
    deliver(app, links_in(std::env::args()));
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_deep_link::DeepLinkExt;
        let handle = app.clone();
        app.deep_link().on_open_url(move |e| {
            deliver(&handle, e.urls().into_iter().map(|u| u.to_string()));
        });
    }
    #[cfg(target_os = "linux")]
    if app.env().appimage.is_some() && crate::paths::sandbox_dir().is_none() {
        use tauri_plugin_deep_link::DeepLinkExt;
        if let Err(e) = app.deep_link().register_all() {
            log::warn!("escouade:// not registered for the AppImage: {e}");
        }
    }
}
