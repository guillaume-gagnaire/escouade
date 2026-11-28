//! The browsers of a project (ESC-657; the page of a launch before, ESC-311): each a webview of its
//! own, a child of the main window placed over the spot the window keeps for it (alone in the main
//! area, or in a pane). One page per agent's launch and per launch command: launching again reloads
//! the same page. The « Pages » window of before is gone: every page is a row of « Navigateurs ».
//!
//! The page is the developed app, a content Escouade does not trust. It is kept apart, without
//! exception:
//! - no command answers it: `refuses` turns away every call of a webview of its own (the window's
//!   thread wrapper, `windowthread`), and the capability names the window's webviews alone, so no
//!   plugin answers it either;
//! - its cookies and storage are its project's, never the window's: a data folder per project
//!   (Windows, Linux), a data store per project (macOS 14 and later), a private session before;
//! - it is a real browser (`judge`, ESC-695): any web origin stays in its tab, and the windows
//!   its page opens (a sign-in's) open in Escouade on its session; Escouade's own addresses, this
//!   machine at a port its project does not declare, files and other schemes are refused, mail
//!   and phone go to the system;
//! - its devtools are its own (`open_devtools` on its webview), opened by a gesture of the user
//!   alone (ESC-657): its bar, a key while Escouade has the focus, the system's own keys and menu
//!   in the page; never by an address the page could go to by itself. The window's stay off in a
//!   release.
//! - what the core reads of it (`browserread`, ESC-668) is read in a world of Escouade's own, by
//!   the webview's native handle: the page has no channel to Escouade either way.
//!
//! A page is keyed by the agent whose launch opened it, by `run:<project>:<command>` for a launch
//! command run in its project (ESC-560), or by `browser:<id>` for one the user opened alone.

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(feature = "desktop")]
use tauri::webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::Url;
#[cfg(feature = "desktop")]
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, WebviewUrl};

/// The labels of the pages' webviews: `devpage-<agent>`.
pub const PAGE_PREFIX: &str = "devpage-";
/// The key of a browser the user opened alone, without an agent (ESC-657): `browser:<id>`.
pub const BROWSER_PREFIX: &str = "browser:";
/// The key of the page of a launch command run in its project, not in a worktree (ESC-560):
/// `run:<project>:<command>`. It has no agent: a browser's row of its own (ESC-657).
pub const RUN_PREFIX: &str = "run:";
/// The address a page goes to, by its shortcut (Ctrl+Shift+M, Cmd+Shift+M), to be shown at the
/// next width (`Verdict::Responsive`).
pub const RESPONSIVE_URL: &str = "escouade-devtools://responsive/";
/// The address a window of a page goes to when its page closes it (`window.close`, ESC-695): the
/// window closes. Only its own page goes there, which may close it anyway; a tab ignores it.
pub const CLOSE_URL: &str = "escouade-devtools://close/";

/// What a window of a page runs first: its `window.close` closes it, on every system (WKWebView
/// does not tell wry when its page closes it).
pub fn close_script() -> String {
    format!(
        "(function () {{ try {{\
           var go = function () {{ window.location.href = '{CLOSE_URL}'; }};\
           Object.defineProperty(window, 'close', {{ value: go, configurable: true, writable: true }});\
         }} catch (e) {{}} }})();"
    )
}

/// The most captures an agent takes in a minute (`capture_browser`, ESC-670).
pub const SHOTS_PER_MINUTE: usize = 20;

/// The width a page is shown at: the whole room, a tablet's, a phone's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageMode {
    #[default]
    Full,
    Tablet,
    Mobile,
}

/// Where a page is on screen, as the window placed it (`list_browsers`, ESC-669): alone in the main
/// area, in a pane beside or under something else, or nowhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    Full,
    Beside,
    Below,
    #[default]
    Hidden,
}

/// An agent's leave to read an origin in one browser (ESC-669, choice C4: outside the project's
/// launch, once per origin and per browser), or its request of it. It falls when the browser
/// closes, or when the user takes the leaves back (« Retirer les accords »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Consent {
    /// The agent's id.
    pub agent: String,
    /// `scheme://host[:port]`.
    pub origin: String,
}

/// The leaves of a browser and what waits for the user's answer, for its band.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Consents {
    pub asks: Vec<Consent>,
    pub grants: Vec<Consent>,
}

/// What an agent gets when it would read an origin of a browser outside the launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leave {
    Granted,
    /// Asked of the user now: the band shows it.
    Asked,
    /// Already asked, the answer still to come.
    Waiting,
    /// The user said no, until the browser closes or the leaves are taken back.
    Refused,
}

/// The most requests a browser holds at once, of every agent: the band shows them one by one.
pub const ASKS_MAX: usize = 5;

/// The most windows the page of a tab has open at once (a sign-in's, an OAuth's, ESC-695): the
/// next is refused, and the window says so.
pub const POPUPS_MAX: usize = 3;

/// Keeps a label to what Tauri allows (letters, digits, `-`, `_`), every other character an `_`.
fn clean(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The label of the page of agent `agent_id`.
pub fn page_label(agent_id: &str) -> String {
    format!("{PAGE_PREFIX}{}", clean(agent_id))
}

/// Whether `key` may name a page without an agent in project `project_id` (ESC-657): a browser the
/// user opened alone, `browser:` and an id of letters, digits, `-` and `_` (40 at most); or the
/// page of a launch command run in this project (ESC-560), `run:<project>:<command>`.
pub fn agentless_key(key: &str, project_id: &str) -> Result<(), String> {
    if let Some(command) = key.strip_prefix(&format!("{RUN_PREFIX}{project_id}:")) {
        return if command.is_empty() {
            Err("not the page of a launch of this project".into())
        } else {
            Ok(())
        };
    }
    if key.starts_with(RUN_PREFIX) {
        return Err("not the page of a launch of this project".into());
    }
    let id = key
        .strip_prefix(BROWSER_PREFIX)
        .ok_or("not the key of a browser")?;
    let fine = !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if fine {
        Ok(())
    } else {
        Err("not the key of a browser".into())
    }
}

/// A call from the webview `label` that no command answers: one of a page's.
pub fn refuses(label: &str) -> bool {
    label.starts_with(PAGE_PREFIX)
}

/// What a page asked to load leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// A web address (or a blank page): the page loads it, in its tab (ESC-695).
    Here,
    /// A mail or phone address: refused, the system's mail client or phone opens it.
    Outside,
    /// Its shortcut for the next width: refused, and the window shows it so.
    Responsive,
    /// Escouade's own pages, this machine at a port the project does not declare, a file, a script,
    /// another scheme: refused, opened nowhere.
    Nowhere,
}

/// Whether a port of this machine is one the project's launch declares (its launch commands, its
/// agents' blocks and test launches), never Escouade's server's (`mcp::browsers::local_of`).
pub type Local = Arc<dyn Fn(u16) -> bool + Send + Sync>;

/// Where `url`, asked by the page of the tab that opened at `home` (a link, a redirection, a form,
/// its bar, a window it opens), leads. A tab is a real browser (ESC-695): any web origin stays in
/// it, where a sign-in goes through its provider and comes back. Never there: Escouade's own
/// addresses (`own`, `navigation::destination`, even on the tab's origin: the dev server's, when
/// the launch is Escouade itself); this machine (`mcp::browsers::is_loopback`) at a port that is
/// neither the tab's own nor one the project declares (`local`), where a site would reach a service
/// of the machine; a file, a script, `data:`, `blob:` and any other scheme. Mail and phone open in
/// the system. What an agent opens is judged before, by `open_browser` (C3).
pub fn judge(
    url: &Url,
    home: &Url,
    own: impl Fn(&Url) -> bool,
    local: impl Fn(u16) -> bool,
) -> Verdict {
    if url.as_str() == RESPONSIVE_URL {
        return Verdict::Responsive;
    }
    if own(url) {
        return Verdict::Nowhere;
    }
    if url.as_str() == "about:blank" {
        return Verdict::Here;
    }
    match url.scheme() {
        "http" | "https" if url.host_str().is_none_or(str::is_empty) => Verdict::Nowhere,
        "http" | "https" if crate::mcp::browsers::is_loopback(url) => {
            let port = url.port_or_known_default();
            let its_own = crate::mcp::browsers::is_loopback(home)
                && port.is_some()
                && port == home.port_or_known_default();
            if url.origin() == home.origin() || its_own || port.is_some_and(&local) {
                Verdict::Here
            } else {
                Verdict::Nowhere
            }
        }
        "http" | "https" => Verdict::Here,
        "mailto" | "tel" => Verdict::Outside,
        _ => Verdict::Nowhere,
    }
}

/// The address of a launch a page may open: http or https, never one of Escouade's own.
pub fn launch_url(text: &str, own: impl Fn(&Url) -> bool) -> Result<Url, String> {
    let url = Url::parse(text.trim()).map_err(|e| format!("not an address: {e}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none_or(str::is_empty) {
        return Err(format!("not a web address: {}:", url.scheme()));
    }
    if own(&url) {
        return Err("Escouade's own address".into());
    }
    Ok(url)
}

/// The folder of the cookies and storage of the pages of project `project_id`.
pub fn data_dir(root: &Path, project_id: &str) -> PathBuf {
    root.join("pages").join(clean(project_id))
}

/// The data store of the pages of project `project_id` (WKWebView, macOS 14 and later): the first
/// 16 bytes of the SHA-256 of its id, the same for its pages, another for each project.
// Read by `with_store` on macOS alone; the tests run it everywhere.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn store_id(project_id: &str) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(format!("escouade-pages:{project_id}").as_bytes());
    let mut id = [0u8; 16];
    id.copy_from_slice(&hash[..16]);
    id
}

/// What the page's shortcut runs in it: Ctrl+Shift+M (Cmd+Shift+M) asks for the next width by
/// `RESPONSIVE_URL`. Nothing else is added to the page. Its devtools have no such shortcut
/// (ESC-657): an address the page's own script could go to, without a gesture of the user. In the
/// page, the system's own keys open them where it has some (F12 and Ctrl+Shift+I in WebView2), and
/// its menu everywhere (« Inspecter l'élément »).
pub fn shortcut_script() -> String {
    format!(
        "window.addEventListener('keydown', function (e) {{\
           var k = (e.key || '').toLowerCase();\
           var m = k === 'm' || e.code === 'KeyM';\
           if (m && e.shiftKey && !e.altKey && (e.ctrlKey || e.metaKey)) {{\
             e.preventDefault(); e.stopPropagation(); window.location.href = '{RESPONSIVE_URL}';\
           }}\
         }}, true);"
    )
}

/// Where a page is placed, in the window's logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A page, as the window shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageView {
    pub agent_id: String,
    pub project_id: String,
    /// What the tab says: the worktree and the process (« pagination-users · web »).
    pub title: String,
    /// Where it is now.
    pub url: String,
    /// The width it is shown at.
    pub mode: PageMode,
    /// Its last load finished (ESC-657): what the window reads at its opening, whose events may
    /// have come before it knew the page.
    #[serde(default)]
    pub loaded: bool,
    /// Its document's title, once the page gave one (`clip_title`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc_title: String,
    /// Where the window shows it (ESC-669).
    #[serde(default)]
    pub placement: Placement,
    /// The agent that opened it by `open_browser` (ESC-678): a `browser:<id>` it may close. None
    /// for one the user opened, an agent's test page or a launch command's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opener: Option<String>,
    /// The agent that drives it (ESC-678, `open_browser` until `close_browser`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    /// The order it opened in, the oldest first: its row's place in « Navigateurs ».
    #[serde(skip)]
    pub opened: u64,
}

/// Where an agent wants a browser it opens (`open_browser`, ESC-678): alone in the main area,
/// beside its conversation, under it, or its row alone without showing anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Wish {
    Full,
    #[default]
    Beside,
    Below,
    Quiet,
}

/// An agent's request to open an address outside its project's launch (choice C3, ESC-678): the
/// user answers it in a band of the agent's conversation, « Permettre une fois » or « Refuser ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAsk {
    /// `o-<n>`.
    pub id: String,
    pub agent: String,
    pub project_id: String,
    /// `scheme://host[:port]`.
    pub origin: String,
    pub url: String,
    pub wish: Wish,
}

/// A browser an agent opened or showed again (`open_browser`, ESC-678), for the window: its row,
/// and where it goes (`place`: `full`, `beside`, `below`, `waiting` until the user looks at the
/// agent's conversation, or `quiet`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub view: PageView,
    pub agent: String,
    pub wish: Wish,
    pub place: &'static str,
}

/// The most openings an agent makes in an hour (asked ones included).
pub const OPENS_PER_HOUR: usize = 10;
/// The most browsers an agent drives at once.
pub const DRIVEN_MAX: usize = 3;
/// The most requests to open an address an agent has waiting for the user.
pub const OPEN_ASKS_MAX: usize = 5;

/// What the agents asked to open, what the user refused them, how often they opened, and the
/// origins the user let open in each browser (ESC-678).
#[derive(Default)]
struct Openings {
    asks: Vec<OpenAsk>,
    /// (agent, origin) the user refused to open: not asked again.
    refused: std::collections::HashSet<(String, String)>,
    /// When each agent opened, the last hour's.
    times: HashMap<String, std::collections::VecDeque<std::time::Instant>>,
    /// By browser: the origins outside the launch the user let open in it. They fall with it.
    allowed: HashMap<String, Vec<String>>,
    seq: u64,
}

/// What an agent gets when it would open an origin outside its project's launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenLeave {
    /// Asked of the user now (its id).
    Asked(String),
    /// Already asked, the answer still to come.
    Waiting(String),
    /// The user said no.
    Refused,
    /// `OPEN_ASKS_MAX` of its requests wait already.
    Full,
}

/// What happened to a page, for the window (`UiEvent::DevPage`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageEvent {
    pub agent_id: String,
    /// `started` and `finished` (a load, at `url`), `outside` (`url`, a mail or phone address,
    /// opened by the system), `refused` (`url` opened nowhere), `popups` (a window of the page
    /// refused: `POPUPS_MAX` open), `responsive` (its width's shortcut), `title`
    /// (its document's title changed, in `title`), `consents` (an agent asked to read it, or its
    /// leaves changed: the window reads them again, `devpage_consents`).
    pub phase: &'static str,
    pub url: String,
    /// Its document's title (`title`), cleaned by `clip_title`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
}

/// Tells a page's events.
pub type Tell = Arc<dyn Fn(PageEvent) + Send + Sync>;

/// The pages open, by agent.
#[derive(Default)]
pub struct Pages {
    pages: Mutex<HashMap<String, PageView>>,
    /// The console and the network's failures of each page, kept from its webview's creation
    /// (`browserread`, ESC-668), by the same key.
    logs: Mutex<HashMap<String, Arc<crate::browserread::Buffers>>>,
    /// The leaves to read each page outside the launch, its requests and its refusals, by key
    /// (ESC-669): they fall with the page.
    consents: Mutex<HashMap<String, Leaves>>,
    /// What the agents asked to open and opened (ESC-678).
    openings: Mutex<Openings>,
    /// The agents whose conversation the window shows now (ESC-678): an agent's browser opens
    /// beside its conversation only while the user looks at it, else its row waits.
    looking: Mutex<std::collections::HashSet<String>>,
    /// The agents whose pane the user closed (AB-9), with whether the user has left their conversation since: nothing
    /// of theirs opens on screen until the user leaves it and comes back.
    dismissed: Mutex<HashMap<String, bool>>,
    /// The order the pages opened in.
    seq: std::sync::atomic::AtomicU64,
    /// The captures each agent took in the last minute (`capture_browser`, ESC-670), by agent.
    shots: Mutex<HashMap<String, std::collections::VecDeque<std::time::Instant>>>,
    /// The driving of each row (`drive_browser`, ESC-679): the hand, the pace, the leaves to fill a password, by the
    /// same key; agents' pages (`page:<p-n>`) among them. It falls with the row.
    pub drives: crate::browserdrive::Drives,
    /// The windows the page of each tab opened (ESC-695), by its key: their labels.
    popups: Mutex<HashMap<String, Vec<String>>>,
}

/// What the agents may read of one page outside the launch.
#[derive(Default)]
struct Leaves {
    asks: Vec<Consent>,
    grants: Vec<Consent>,
    refused: Vec<Consent>,
}

impl Pages {
    /// One more window of the page of tab `key`, `label`: refused when it has `POPUPS_MAX`.
    pub fn add_popup(&self, key: &str, label: &str) -> bool {
        let mut all = self.popups.lock();
        let list = all.entry(key.to_string()).or_default();
        if list.len() >= POPUPS_MAX {
            return false;
        }
        list.push(label.to_string());
        true
    }

    /// The window `label` is gone.
    pub fn remove_popup(&self, label: &str) {
        let mut all = self.popups.lock();
        for list in all.values_mut() {
            list.retain(|l| l != label);
        }
        all.retain(|_, l| !l.is_empty());
    }

    /// How many windows the page of tab `key` has open.
    pub fn popups(&self, key: &str) -> usize {
        self.popups.lock().get(key).map_or(0, Vec::len)
    }

    /// The windows of the page of tab `key`, which closes: forgotten.
    pub fn take_popups(&self, key: &str) -> Vec<String> {
        self.popups.lock().remove(key).unwrap_or_default()
    }

    pub fn get(&self, agent_id: &str) -> Option<PageView> {
        self.pages.lock().get(agent_id).cloned()
    }

    /// Every page open, the oldest first.
    pub fn all(&self) -> Vec<PageView> {
        let mut all: Vec<PageView> = self.pages.lock().values().cloned().collect();
        all.sort_by_key(|p| p.opened);
        all
    }

    /// `page` open (or opened again: it keeps its place).
    pub(crate) fn put(&self, mut page: PageView) {
        let mut pages = self.pages.lock();
        page.opened = match pages.get(&page.agent_id) {
            Some(before) => before.opened,
            None => self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1,
        };
        pages.insert(page.agent_id.clone(), page);
    }

    /// Where the window shows the page `key` now.
    pub fn set_placement(&self, key: &str, placement: Placement) {
        if let Some(p) = self.pages.lock().get_mut(key) {
            p.placement = placement;
        }
    }

    /// Whether `agent` may read `origin` in the page `key`, or asks the user (ESC-669). A request
    /// beyond `ASKS_MAX` waiting is not kept, and says so as `Waiting`.
    pub fn leave(&self, key: &str, agent: &str, origin: &str) -> Leave {
        let c = Consent {
            agent: agent.to_string(),
            origin: origin.to_string(),
        };
        let mut all = self.consents.lock();
        let leaves = all.entry(key.to_string()).or_default();
        if leaves.grants.contains(&c) {
            Leave::Granted
        } else if leaves.refused.contains(&c) {
            Leave::Refused
        } else if leaves.asks.contains(&c) || leaves.asks.len() >= ASKS_MAX {
            Leave::Waiting
        } else {
            leaves.asks.push(c);
            Leave::Asked
        }
    }

    /// Whether `agent` was let read `origin` in the page `key`.
    pub fn granted(&self, key: &str, agent: &str, origin: &str) -> bool {
        self.consents.lock().get(key).is_some_and(|l| {
            l.grants
                .iter()
                .any(|c| c.agent == agent && c.origin == origin)
        })
    }

    /// The user's answer to the request of `agent` to read `origin` in the page `key`: false when
    /// none waits (nothing changes then).
    pub fn answer(&self, key: &str, agent: &str, origin: &str, allow: bool) -> bool {
        let mut all = self.consents.lock();
        let Some(leaves) = all.get_mut(key) else {
            return false;
        };
        let Some(at) = leaves
            .asks
            .iter()
            .position(|c| c.agent == agent && c.origin == origin)
        else {
            return false;
        };
        let c = leaves.asks.remove(at);
        if allow {
            leaves.grants.push(c);
        } else {
            leaves.refused.push(c);
        }
        true
    }

    /// « Retirer les accords » of the page `key`: its leaves, its requests and its refusals all
    /// gone. The agents whose request waited, to be told.
    pub fn revoke(&self, key: &str) -> Vec<Consent> {
        self.consents
            .lock()
            .remove(key)
            .map(|l| l.asks)
            .unwrap_or_default()
    }

    /// What the band of the page `key` shows.
    pub fn consents(&self, key: &str) -> Consents {
        self.consents
            .lock()
            .get(key)
            .map(|l| Consents {
                asks: l.asks.clone(),
                grants: l.grants.clone(),
            })
            .unwrap_or_default()
    }

    /// The agents whose conversation the window shows now.
    pub fn set_looking(&self, agents: impl IntoIterator<Item = String>) {
        let now: std::collections::HashSet<String> = agents.into_iter().collect();
        self.dismissed.lock().retain(|agent, away| {
            if !now.contains(agent) {
                *away = true;
                true
            } else {
                !*away
            }
        });
        *self.looking.lock() = now;
    }

    /// The user closed the pane of a browser or a page \`agent\` drives, or its row (AB-9): nothing of the agent's
    /// opens on screen until the user has left its conversation and come back to it.
    pub fn dismiss(&self, agent: &str) {
        let away = !self.looking(agent);
        self.dismissed.lock().insert(agent.to_string(), away);
    }

    /// Whether the user closed the pane of \`agent\` and has not come back to its conversation since.
    pub fn dismissed(&self, agent: &str) -> bool {
        self.dismissed.lock().contains_key(agent)
    }

    /// Whether the user looks at the conversation of `agent`.
    pub fn looking(&self, agent: &str) -> bool {
        self.looking.lock().contains(agent)
    }

    /// The agent that opened the page `key` (`open_browser`): its own to close.
    pub fn set_opener(&self, key: &str, agent: &str) {
        if let Some(p) = self.pages.lock().get_mut(key) {
            p.opener = Some(agent.to_string());
        }
    }

    /// The agent that drives the page `key`, or none (`close_browser`).
    pub fn set_driver(&self, key: &str, agent: Option<&str>) {
        if let Some(p) = self.pages.lock().get_mut(key) {
            p.driver = agent.map(str::to_string);
        }
    }

    /// How many pages `agent` drives, but `except`.
    pub fn driven_by(&self, agent: &str, except: &str) -> usize {
        self.pages
            .lock()
            .values()
            .filter(|p| p.agent_id != except && p.driver.as_deref() == Some(agent))
            .count()
    }

    /// Whether `agent` may open once more this hour (`OPENS_PER_HOUR`).
    pub fn may_open(&self, agent: &str) -> bool {
        let mut o = self.openings.lock();
        let times = o.times.entry(agent.to_string()).or_default();
        let hour = std::time::Duration::from_secs(3600);
        while times.front().is_some_and(|t| t.elapsed() >= hour) {
            times.pop_front();
        }
        times.len() < OPENS_PER_HOUR
    }

    /// `agent` opened (or asked to) now.
    pub fn note_open(&self, agent: &str) {
        self.openings
            .lock()
            .times
            .entry(agent.to_string())
            .or_default()
            .push_back(std::time::Instant::now());
    }

    /// Asks the user to let `agent` open `url` (of `origin`), unless it asked already, was
    /// refused, or has `OPEN_ASKS_MAX` requests waiting.
    pub fn ask_open(
        &self,
        agent: &str,
        project_id: &str,
        origin: &str,
        url: &str,
        wish: Wish,
    ) -> OpenLeave {
        let mut o = self.openings.lock();
        if o.refused.contains(&(agent.to_string(), origin.to_string())) {
            return OpenLeave::Refused;
        }
        if let Some(a) = o
            .asks
            .iter()
            .find(|a| a.agent == agent && a.origin == origin)
        {
            return OpenLeave::Waiting(a.id.clone());
        }
        if o.asks.iter().filter(|a| a.agent == agent).count() >= OPEN_ASKS_MAX {
            return OpenLeave::Full;
        }
        o.seq += 1;
        let id = format!("o-{}", o.seq);
        o.asks.push(OpenAsk {
            id: id.clone(),
            agent: agent.to_string(),
            project_id: project_id.to_string(),
            origin: origin.to_string(),
            url: url.to_string(),
            wish,
        });
        OpenLeave::Asked(id)
    }

    /// The requests to open that wait for the user, the oldest first.
    pub fn open_asks(&self) -> Vec<OpenAsk> {
        self.openings.lock().asks.clone()
    }

    /// The user's answer to the request `id`: the request, gone; none when it waits no more. A
    /// refusal holds for that agent and origin.
    pub fn answer_open(&self, id: &str, allow: bool) -> Option<OpenAsk> {
        let mut o = self.openings.lock();
        let at = o.asks.iter().position(|a| a.id == id)?;
        let ask = o.asks.remove(at);
        if !allow {
            o.refused.insert((ask.agent.clone(), ask.origin.clone()));
        }
        Some(ask)
    }

    /// The requests of `agent`, gone with it.
    pub fn forget_open_asks(&self, agent: &str) -> Vec<OpenAsk> {
        let mut o = self.openings.lock();
        let (gone, kept) = std::mem::take(&mut o.asks)
            .into_iter()
            .partition(|a| a.agent == agent);
        o.asks = kept;
        gone
    }

    /// The user let `origin` open in the page `key`.
    pub fn allow_origin(&self, key: &str, origin: &str) {
        let mut o = self.openings.lock();
        let list = o.allowed.entry(key.to_string()).or_default();
        if !list.iter().any(|x| x == origin) {
            list.push(origin.to_string());
        }
    }

    /// Whether the user let `origin` open in the page `key`.
    pub fn origin_allowed(&self, key: &str, origin: &str) -> bool {
        self.openings
            .lock()
            .allowed
            .get(key)
            .is_some_and(|l| l.iter().any(|x| x == origin))
    }

    fn set_url(&self, agent_id: &str, url: &str) {
        if let Some(p) = self.pages.lock().get_mut(agent_id) {
            p.url = url.to_string();
        }
    }

    fn set_loaded(&self, agent_id: &str, loaded: bool) {
        if let Some(p) = self.pages.lock().get_mut(agent_id) {
            p.loaded = loaded;
        }
    }

    fn set_doc_title(&self, agent_id: &str, title: &str) {
        if let Some(p) = self.pages.lock().get_mut(agent_id) {
            p.doc_title = title.to_string();
        }
    }

    /// One more capture by `agent` at `now`, `SHOTS_PER_MINUTE` in a minute at most: refused, the
    /// seconds to wait until the oldest of the minute leaves it.
    pub fn take_shot(&self, agent: &str, now: std::time::Instant) -> Result<(), u64> {
        let minute = std::time::Duration::from_secs(60);
        let mut shots = self.shots.lock();
        let taken = shots.entry(agent.to_string()).or_default();
        while taken
            .front()
            .is_some_and(|t| now.saturating_duration_since(*t) >= minute)
        {
            taken.pop_front();
        }
        if taken.len() >= SHOTS_PER_MINUTE {
            let oldest = taken.front().copied().unwrap_or(now);
            let left = minute.saturating_sub(now.saturating_duration_since(oldest));
            return Err(left.as_secs().max(1));
        }
        taken.push_back(now);
        Ok(())
    }

    /// Shows the page of `agent_id` at `mode`. False when it is not open.
    pub fn set_mode(&self, agent_id: &str, mode: PageMode) -> bool {
        match self.pages.lock().get_mut(agent_id) {
            Some(p) => {
                p.mode = mode;
                true
            }
            None => false,
        }
    }

    fn remove(&self, agent_id: &str) -> Option<PageView> {
        if let Some(logs) = self.logs.lock().remove(agent_id) {
            logs.close();
        }
        // Its leaves fall with it (ESC-669), and the origins the user let open in it (ESC-678).
        self.consents.lock().remove(agent_id);
        self.openings.lock().allowed.remove(agent_id);
        self.drives.forget(agent_id);
        self.pages.lock().remove(agent_id)
    }

    /// The console and the network's failures of the page `agent_id`, kept since it opened.
    pub fn logs(&self, agent_id: &str) -> Option<Arc<crate::browserread::Buffers>> {
        self.logs.lock().get(agent_id).cloned()
    }

    /// New buffers for the page `agent_id`, its webview just made.
    pub(crate) fn new_logs(&self, agent_id: &str) -> Arc<crate::browserread::Buffers> {
        let logs = Arc::new(crate::browserread::Buffers::default());
        if let Some(old) = self.logs.lock().insert(agent_id.to_string(), logs.clone()) {
            old.close();
        }
        logs
    }
}

/// What opening a page needs.
pub struct Open<'a> {
    pub agent_id: &'a str,
    pub project_id: &'a str,
    pub title: &'a str,
    pub url: &'a str,
    /// Where it goes in the main window, when it is there (hidden until placed otherwise).
    pub bounds: Option<Bounds>,
    /// Escouade's data folder (`data_dir`).
    pub data_root: &'a Path,
    /// Shown at once, else hidden until `place` shows it.
    pub visible: bool,
    /// The ports of this machine its project declares (`judge`).
    pub local: Local,
}

/// Whether an address is Escouade's own, for the main window's setup.
pub type Own = Arc<dyn Fn(&Url) -> bool + Send + Sync>;

/// Whether an address is Escouade's own (`navigation::destination`), never loaded in a page.
#[cfg(feature = "desktop")]
pub fn own_of<R: Runtime>(app: &AppHandle<R>) -> Own {
    let dev = app
        .try_state::<crate::navigation::Home>()
        .and_then(|h| h.dev.clone());
    Arc::new(move |u: &Url| {
        crate::navigation::destination(u, dev.as_ref()) == crate::navigation::Destination::App
    })
}

/// Opens the page of agent `o.agent_id` at `o.url`, or, when it is open, loads `o.url` in it
/// (launching again reloads the same page).
#[cfg(feature = "desktop")]
pub fn open<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Arc<Pages>,
    o: Open<'_>,
    own: Own,
    tell: Tell,
) -> Result<(), String> {
    let url = launch_url(o.url, |u| own(u))?;
    let label = page_label(o.agent_id);
    if let Some(webview) = app.get_webview(&label) {
        let same = webview.url().ok().is_some_and(|u| u == url);
        if same {
            webview.reload().map_err(|e| e.to_string())?;
        } else {
            webview.navigate(url.clone()).map_err(|e| e.to_string())?;
        }
        pages.set_url(o.agent_id, url.as_str());
        pages.set_loaded(o.agent_id, false);
        return Ok(());
    }
    let builder = builder(
        app,
        pages,
        &label,
        o.agent_id,
        o.project_id,
        url.clone(),
        o.data_root,
        own,
        o.local.clone(),
        tell,
    );
    let page = PageView {
        agent_id: o.agent_id.to_string(),
        project_id: o.project_id.to_string(),
        title: o.title.to_string(),
        url: url.to_string(),
        mode: PageMode::Full,
        loaded: false,
        doc_title: String::new(),
        placement: if o.visible && o.bounds.is_some() {
            Placement::Full
        } else {
            Placement::Hidden
        },
        opener: None,
        driver: None,
        opened: 0,
    };
    let window = app.get_window("main").ok_or("no main window")?;
    let b = o.bounds.unwrap_or(Bounds {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    let webview = window
        .add_child(
            builder,
            LogicalPosition::new(b.x, b.y),
            LogicalSize::new(b.width.max(1.0), b.height.max(1.0)),
        )
        .map_err(|e| e.to_string())?;
    if !(o.visible && o.bounds.is_some()) {
        let _ = webview.hide();
    }
    // Its console and its network's failures, from now on (ESC-668).
    let logs = pages.new_logs(o.agent_id);
    logs.set_origin(url.as_str());
    crate::browserread::attach(&webview, logs);
    pages.put(page);
    Ok(())
}

/// What judges every address of a tab and of the windows its page opens (ESC-695): the same rule,
/// the same session, the same events.
#[cfg(feature = "desktop")]
struct Guard<R: Runtime> {
    app: AppHandle<R>,
    pages: Arc<Pages>,
    /// The tab's key.
    key: String,
    /// The tab's address of departure: its own origin and port (`judge`).
    home: Url,
    own: Own,
    local: Local,
    tell: Tell,
}

#[cfg(feature = "desktop")]
impl<R: Runtime> Guard<R> {
    /// Whether the page may load `to`: else what it leads to is done (`act`).
    fn allows(&self, to: &Url) -> bool {
        // A tab's own `window.close` (WKWebView and WebKitGTK may share the windows' scripts with
        // their tab): nothing to close, nothing to say.
        if to.as_str() == CLOSE_URL {
            return false;
        }
        let verdict = judge(to, &self.home, |u| (self.own)(u), |p| (self.local)(p));
        if verdict != Verdict::Here {
            act(&self.key, to, verdict, &self.tell);
        }
        verdict == Verdict::Here
    }
}

/// What a window the page asks for (`window.open`, `target=_blank`, a sign-in's) leads to: in
/// Escouade, on the tab's session, kept as the page's (`window.opener`); refused as any address
/// when `judge` refuses it, or when the tab has `POPUPS_MAX` open.
#[cfg(feature = "desktop")]
fn on_new_window<R: Runtime>(
    g: Arc<Guard<R>>,
) -> impl Fn(Url, tauri::webview::NewWindowFeatures) -> NewWindowResponse<R> + Send + 'static {
    move |to, features| {
        if !g.allows(&to) {
            return NewWindowResponse::Deny;
        }
        match popup(&g, &to, features) {
            Ok(window) => NewWindowResponse::Create { window },
            Err(e) => {
                log::info!("a page's window was not opened: {e}");
                NewWindowResponse::Deny
            }
        }
    }
}

/// The label of a window the page of tab `key` opened: its tab's, then `:w<n>` (a `:` that no
/// tab's label holds), so that no command answers it either (`refuses`).
pub fn popup_label(key: &str, n: u64) -> String {
    format!("{}:w{n}", page_label(key))
}

/// The size of a window the page opens, in logical pixels: what it asked, bounded (320 to 1200
/// wide, 320 to 1000 high: a page asks any number), 520 × 680 when it asked none.
pub fn popup_size(asked: Option<(f64, f64)>) -> (f64, f64) {
    let bound = |v: f64, max: f64| {
        if v.is_finite() {
            v.clamp(320.0, max)
        } else {
            max
        }
    };
    match asked {
        Some((w, h)) => (bound(w, 1200.0), bound(h, 1000.0)),
        None => (520.0, 680.0),
    }
}

/// A window's title: the origin it is on first, which the page does not choose, then its
/// document's title (`clip_title`).
pub fn popup_title(url: &Url, doc: &str) -> String {
    let origin = match url.scheme() {
        "http" | "https" => url.origin().ascii_serialization(),
        _ => String::new(),
    };
    let doc = clip_title(doc);
    match (origin.is_empty(), doc.is_empty()) {
        (true, _) => doc,
        (false, true) => origin,
        (false, false) => format!("{origin} — {doc}"),
    }
}

/// Opens a window of the page of tab `g.key`, for `to`: a window of Escouade's own on the same
/// session as the tab (WebView2's environment, WebKitGTK's related view, WKWebView's configuration:
/// `window_features`), never on top of the other applications, read by nothing of Escouade.
#[cfg(feature = "desktop")]
fn popup<R: Runtime>(
    g: &Arc<Guard<R>>,
    to: &Url,
    features: tauri::webview::NewWindowFeatures,
) -> Result<tauri::WebviewWindow<R>, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let label = popup_label(
        &g.key,
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    );
    if !g.pages.add_popup(&g.key, &label) {
        (g.tell)(PageEvent {
            agent_id: g.key.clone(),
            phase: "popups",
            url: String::new(),
            title: String::new(),
        });
        return Err(format!("{POPUPS_MAX} windows open already"));
    }
    let blank = Url::parse("about:blank").map_err(|e| e.to_string())?;
    // Its size, bounded; its place, over Escouade's window: never where the page says (WebView2
    // gives `left` and `top` unsigned, a negative one huge, which overflows the window's maker).
    let asked = features.size().map(|s| (s.width, s.height));
    let (width, height) = popup_size(asked);
    let at = g.app.get_window("main").and_then(|w| {
        let scale = w.scale_factor().ok()?;
        let pos = w.outer_position().ok()?.to_logical::<f64>(scale);
        let size = w.outer_size().ok()?.to_logical::<f64>(scale);
        Some((
            pos.x + ((size.width - width) / 2.0).max(0.0),
            pos.y + ((size.height - height) / 2.0).max(0.0),
        ))
    });
    let (x, y) = at.unwrap_or((120.0, 120.0));
    let mut b = tauri::WebviewWindowBuilder::new(&g.app, &label, WebviewUrl::External(blank))
        .title(popup_title(to, ""))
        // The tab's session (its environment, related view or configuration), then its own bounds.
        .window_features(features)
        .inner_size(width, height)
        .position(x, y)
        .devtools(true)
        .always_on_top(false)
        .focused(true);
    // Windows: owned by Escouade's window, above it and never above another application.
    #[cfg(windows)]
    {
        if let Some(hwnd) = g.app.get_window("main").and_then(|w| w.hwnd().ok()) {
            b = b.owner_raw(hwnd);
        }
    }
    {
        let (g, label) = (g.clone(), label.clone());
        b = b.on_navigation(move |to| {
            if to.as_str() == CLOSE_URL {
                close_popup(&g.app, &label);
                return false;
            }
            g.allows(to)
        });
    }
    b = b.initialization_script(close_script());
    b = b.on_new_window(on_new_window(g.clone()));
    b = b.on_page_load(|w, load| {
        if matches!(load.event(), PageLoadEvent::Started) {
            let _ = w.set_title(&popup_title(load.url(), ""));
        }
    });
    b = b.on_document_title_changed(|w, title| {
        if let Ok(url) = w.url() {
            let _ = w.set_title(&popup_title(&url, &title));
        }
    });
    let window = match b.build() {
        Ok(w) => w,
        Err(e) => {
            g.pages.remove_popup(&label);
            return Err(e.to_string());
        }
    };
    {
        let (pages, label) = (g.pages.clone(), label.clone());
        window.on_window_event(move |e| {
            if matches!(e, tauri::WindowEvent::Destroyed) {
                pages.remove_popup(&label);
            }
        });
    }
    follow_close(&window);
    log::info!("a page's window opened on its tab's session: {label}");
    Ok(window)
}

/// Closes the window `label` of a page, off the webview's own call (it is destroyed after it).
#[cfg(feature = "desktop")]
fn close_popup<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        tauri::async_runtime::spawn(async move {
            let _ = w.destroy();
        });
    }
}

/// A window of a page closes when its webview is asked to close (its page's `window.close`, or the
/// opener's `w.close()`): wry takes its webview away alone, and the window would stay, empty.
#[cfg(feature = "desktop")]
fn follow_close<R: Runtime>(window: &tauri::WebviewWindow<R>) {
    let (app, label) = (window.app_handle().clone(), window.label().to_string());
    if let Err(e) = window.with_webview(move |webview| on_close(webview, app, label)) {
        log::warn!("a page's window will not close with its page: {e}");
    }
}

#[cfg(windows)]
#[cfg(feature = "desktop")]
fn on_close<R: Runtime>(
    webview: tauri::webview::PlatformWebview,
    app: AppHandle<R>,
    label: String,
) {
    use webview2_com::WindowCloseRequestedEventHandler;
    let handler = WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
        close_popup(&app, &label);
        Ok(())
    }));
    // SAFETY: the controller is the live one of this WebView, used on its thread (`with_webview`).
    let added = unsafe {
        webview.controller().CoreWebView2().and_then(|core| {
            // The registration lasts as long as the WebView: it is never removed.
            let mut token = 0;
            core.add_WindowCloseRequested(&handler, &mut token)
        })
    };
    if let Err(e) = added {
        log::warn!("a page's window will not close with its page: {e}");
    }
}

#[cfg(target_os = "linux")]
#[cfg(feature = "desktop")]
fn on_close<R: Runtime>(
    webview: tauri::webview::PlatformWebview,
    app: AppHandle<R>,
    label: String,
) {
    // wry answers WebKit's `close` by destroying the webview first, which stops the signal there: the
    // window follows its webview's end instead.
    use gtk::prelude::WidgetExt;
    webview
        .inner()
        .connect_destroy(move |_| close_popup(&app, &label));
}

// macOS: WKWebView tells it to a delegate wry keeps; `close_script` does it for the page itself.
#[cfg(not(any(windows, target_os = "linux")))]
#[cfg(feature = "desktop")]
fn on_close<R: Runtime>(
    _webview: tauri::webview::PlatformWebview,
    _app: AppHandle<R>,
    _label: String,
) {
}

/// The webview of a page: any web origin, its project's storage, no command, its shortcut; the
/// windows it opens on its session (ESC-695).
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "desktop")]
fn builder<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Arc<Pages>,
    label: &str,
    agent_id: &str,
    project_id: &str,
    url: Url,
    data_root: &Path,
    own: Own,
    local: Local,
    tell: Tell,
) -> WebviewBuilder<R> {
    let guard = Arc::new(Guard {
        app: app.clone(),
        pages: pages.clone(),
        key: agent_id.to_string(),
        home: url.clone(),
        own,
        local,
        tell: tell.clone(),
    });
    let mut b = WebviewBuilder::new(label, WebviewUrl::External(url))
        .devtools(true)
        .initialization_script(shortcut_script());
    // WebKit gives the app no console: a relay in the page tells Escouade's world (ESC-668).
    if cfg!(not(windows)) {
        b = b.initialization_script(crate::browserread::RELAY);
        // Nor does its isolated world keep a listener: the user's gestures are counted in the page (ESC-679).
        b = b.initialization_script(crate::browserdrive::GESTURES);
    }
    b = with_store(b, data_root, project_id);
    // Every address it is asked to load.
    {
        let g = guard.clone();
        b = b.on_navigation(move |to| g.allows(to));
    }
    // Its new windows: in Escouade, on its session.
    b = b.on_new_window(on_new_window(guard));
    // Its loads, for its bar; what an agent reads of its console is of the origin it is on.
    {
        let (pages, tell) = (pages.clone(), tell.clone());
        let agent = agent_id.to_string();
        b = b.on_page_load(move |_, load| {
            let url = load.url().to_string();
            if matches!(load.event(), PageLoadEvent::Started) {
                if let Some(logs) = pages.logs(&agent) {
                    logs.set_origin(&url);
                }
            }
            pages.set_url(&agent, &url);
            pages.set_loaded(&agent, matches!(load.event(), PageLoadEvent::Finished));
            tell(PageEvent {
                agent_id: agent.clone(),
                phase: match load.event() {
                    PageLoadEvent::Started => "started",
                    PageLoadEvent::Finished => "finished",
                },
                url,
                title: String::new(),
            });
        });
    }
    // Its document's title, for its row in the sidebar (ESC-657).
    {
        let (pages, tell) = (pages.clone(), tell.clone());
        let agent = agent_id.to_string();
        b = b.on_document_title_changed(move |_, title| {
            let url = pages.get(&agent).map(|p| p.url).unwrap_or_default();
            let title = clip_title(&title);
            pages.set_doc_title(&agent, &title);
            tell(PageEvent {
                agent_id: agent.clone(),
                phase: "title",
                url,
                title,
            });
        });
    }
    b
}

/// A document's title as its row says it: one line, its control and invisible characters gone,
/// 120 characters at most (the page chooses it: Escouade does not trust it).
pub fn clip_title(title: &str) -> String {
    let invisible = |c: char| {
        c.is_control()
            || matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{feff}')
    };
    let one: String = title
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| !invisible(*c))
        .collect();
    let words = one.split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>();
    words.join(" ").chars().take(120).collect()
}

/// A page's storage, its project's: a folder (Windows, Linux), a data store (macOS 14 and later),
/// a private session before (WKWebView has no folder of its own to give it there).
#[cfg(feature = "desktop")]
fn with_store<R: Runtime>(
    b: WebviewBuilder<R>,
    data_root: &Path,
    project_id: &str,
) -> WebviewBuilder<R> {
    #[cfg(target_os = "macos")]
    {
        let _ = data_root;
        let major = match os_info::get().version() {
            os_info::Version::Semantic(major, _, _) => *major,
            _ => 0,
        };
        if major >= 14 {
            b.data_store_identifier(store_id(project_id))
        } else {
            b.incognito(true)
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        b.data_directory(data_dir(data_root, project_id))
    }
}

/// What a refused address of a page does: told to the window, which opens it outside; its devtools; or nothing.
#[cfg(feature = "desktop")]
fn act(agent: &str, to: &Url, verdict: Verdict, tell: &Tell) {
    let phase = match verdict {
        // The window opens it, as its own links (`open_url`).
        Verdict::Outside => "outside",
        // The window knows the widths and keeps the one chosen (and lays out the page by it).
        Verdict::Responsive => "responsive",
        _ => {
            log::info!(
                "a page was refused {}://{}",
                to.scheme(),
                to.host_str().unwrap_or("")
            );
            "refused"
        }
    };
    tell(PageEvent {
        agent_id: agent.to_string(),
        phase,
        url: if verdict == Verdict::Responsive {
            String::new()
        } else {
            to.to_string()
        },
        title: String::new(),
    });
}

/// Where each webview was last placed, by label: what `open_devtools` gives it back on macOS.
fn placed() -> &'static Mutex<HashMap<String, Bounds>> {
    static PLACED: std::sync::OnceLock<Mutex<HashMap<String, Bounds>>> = std::sync::OnceLock::new();
    PLACED.get_or_init(Default::default)
}

/// Places `webview` at `b`, in its window's logical pixels.
#[cfg(feature = "desktop")]
pub(crate) fn set_bounds<R: Runtime>(webview: &tauri::Webview<R>, b: Bounds) -> Result<(), String> {
    placed().lock().insert(webview.label().to_string(), b);
    webview
        .set_position(LogicalPosition::new(b.x, b.y))
        .map_err(|e| e.to_string())?;
    webview
        .set_size(LogicalSize::new(b.width, b.height))
        .map_err(|e| e.to_string())?;
    // WebKitGTK: Tauri 2.12 packs a window's child webviews in a `GtkBox`, one after the other, and
    // only allocates a size once: a page there can neither be placed nor narrowed, and a size
    // requested of it grows the window (seen by `devpage_probe`). It goes into a `gtk::Layout`
    // that takes the place of the window's own webview in the box, and is placed there.
    #[cfg(target_os = "linux")]
    {
        let (x, y) = (b.x.round() as i32, b.y.round() as i32);
        let (w, h) = (
            b.width.round().max(1.0) as i32,
            b.height.round().max(1.0) as i32,
        );
        webview
            .with_webview(move |platform| {
                use gtk::prelude::*;
                let view = platform.inner();
                if let Some(layout) = gtk_layout_of(view.upcast_ref()) {
                    view.set_size_request(w, h);
                    layout.move_(&view, x, y);
                }
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The `gtk::Layout` `view` is placed in: made the first time, in the place of the window's own
/// webview in its `GtkBox` (which then follows the size the layout is given), `view` moved in.
/// A layout asks the window for no size: the window still shrinks as the user wants.
#[cfg(target_os = "linux")]
#[cfg(feature = "desktop")]
fn gtk_layout_of(view: &gtk::Widget) -> Option<gtk::Layout> {
    use gtk::prelude::*;
    let parent = view.parent()?;
    if let Ok(layout) = parent.clone().downcast::<gtk::Layout>() {
        return Some(layout);
    }
    let boxed = parent.downcast::<gtk::Box>().ok()?;
    let children = boxed.children();
    let layout = match children
        .iter()
        .find_map(|c| c.clone().downcast::<gtk::Layout>().ok())
    {
        Some(layout) => layout,
        None => {
            // The window's own webview: the first child that is not this page.
            let own = children.iter().find(|c| *c != view)?.clone();
            let layout = gtk::Layout::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
            boxed.remove(&own);
            boxed.pack_start(&layout, true, true, 0);
            boxed.reorder_child(&layout, 0);
            layout.put(&own, 0, 0);
            let follow = own.clone();
            layout.connect_size_allocate(move |_, room| {
                if follow.allocated_width() != room.width()
                    || follow.allocated_height() != room.height()
                {
                    follow.set_size_request(room.width(), room.height());
                }
            });
            layout.show_all();
            layout
        }
    };
    boxed.remove(view);
    layout.put(view, 0, 0);
    view.show();
    Some(layout)
}

/// The page `agent_id` in the window: placed at `bounds` and shown, or hidden. `placement`: where
/// the window shows it (alone, beside or under something else), `Full` when it does not say.
#[cfg(feature = "desktop")]
pub fn place<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Pages,
    agent_id: &str,
    bounds: Option<Bounds>,
    placement: Option<Placement>,
) -> Result<(), String> {
    if pages.get(agent_id).is_none() {
        return Ok(());
    }
    let Some(webview) = app.get_webview(&page_label(agent_id)) else {
        return Ok(());
    };
    match bounds {
        Some(b) if b.width >= 1.0 && b.height >= 1.0 => {
            set_bounds(&webview, b)?;
            webview.show().map_err(|e| e.to_string())?;
            let shown = match placement {
                Some(Placement::Hidden) | None => Placement::Full,
                Some(p) => p,
            };
            pages.set_placement(agent_id, shown);
            Ok(())
        }
        _ => {
            pages.set_placement(agent_id, Placement::Hidden);
            webview.hide().map_err(|e| e.to_string())
        }
    }
}

/// Back, forward, reload, or an address typed or pasted in its bar (judged by the page's guard like
/// any other: any web origin loads in the tab, a sign-in's link included).
#[cfg(feature = "desktop")]
pub fn go<R: Runtime>(
    app: &AppHandle<R>,
    agent_id: &str,
    action: &str,
    to: Option<&str>,
) -> Result<(), String> {
    let webview = app
        .get_webview(&page_label(agent_id))
        .ok_or("no page for this agent")?;
    match action {
        "back" => webview.eval("history.back()").map_err(|e| e.to_string()),
        "forward" => webview.eval("history.forward()").map_err(|e| e.to_string()),
        "reload" => webview.reload().map_err(|e| e.to_string()),
        "to" => {
            let url = Url::parse(to.unwrap_or("").trim()).map_err(|e| e.to_string())?;
            webview.navigate(url).map_err(|e| e.to_string())
        }
        other => Err(format!("unknown action {other}")),
    }
}

/// Shows the page `agent_id` at `mode` (the window places it by its mode).
pub fn set_mode(pages: &Pages, agent_id: &str, mode: PageMode) -> Result<(), String> {
    if !pages.set_mode(agent_id, mode) {
        return Err("no page for this agent".into());
    }
    Ok(())
}

/// Opens the devtools of the page of agent `agent_id`, never the window's.
#[cfg(feature = "desktop")]
pub fn devtools<R: Runtime>(app: &AppHandle<R>, agent_id: &str) -> Result<(), String> {
    let webview = app
        .get_webview(&page_label(agent_id))
        .ok_or("no page for this agent")?;
    open_devtools(&webview);
    Ok(())
}

/// Opens the devtools of a page. macOS: WebKit's inspector, attached to the page, lays out the
/// page's view itself and gives it back the window's whole width after the window placed it at a
/// tablet's or a phone's (seen by `devpage_probe` in the CI): it goes into a window of its own,
/// as WebView2's devtools are on Windows; and as WebKit gives the view back its former frame when
/// the move ends, later and on its own, the page is placed again where it was, a few times while
/// the inspector settles (asked again to leave the page each time: it may ignore an ask while it
/// is still being shown).
#[cfg(feature = "desktop")]
fn open_devtools<R: Runtime>(webview: &tauri::Webview<R>) {
    webview.open_devtools();
    #[cfg(target_os = "macos")]
    {
        let page = webview.clone();
        std::thread::spawn(move || {
            for wait in [0, 300, 700, 1500, 3000, 5000, 8000] {
                std::thread::sleep(std::time::Duration::from_millis(wait));
                let _ = page.with_webview(|platform| {
                    detach_inspector(platform.inner().cast());
                });
                let last = placed().lock().get(page.label()).copied();
                if let Some(b) = last {
                    let _ = set_bounds(&page, b);
                }
            }
        });
    }
}

/// Asks the inspector of the WKWebView `view` to leave the page for a window of its own. Each
/// call is checked first (`respondsToSelector:`): an Objective-C exception would abort the app.
#[cfg(target_os = "macos")]
#[cfg(feature = "desktop")]
fn detach_inspector(view: *mut objc2::runtime::AnyObject) {
    use objc2::runtime::{AnyObject, Sel};
    if view.is_null() {
        return;
    }
    // SAFETY: `view` is a page's live WKWebView, used on the main thread (`with_webview`); every
    // selector is asked for before it is sent. `_inspector` (macOS 10.14+) answers its
    // `_WKInspector`, whose `detach` is WebKit's (private, as the `devtools` feature itself).
    unsafe {
        let answers = |object: *mut AnyObject, selector: Sel| -> bool {
            objc2::msg_send![object, respondsToSelector: selector]
        };
        if !answers(view, objc2::sel!(_inspector)) {
            return;
        }
        let inspector: *mut AnyObject = objc2::msg_send![view, _inspector];
        if !inspector.is_null() && answers(inspector, objc2::sel!(detach)) {
            let _: () = objc2::msg_send![inspector, detach];
        }
    }
}

/// Closes the page of agent `agent_id` (its agent gone, its ticket done).
#[cfg(feature = "desktop")]
pub fn close<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Arc<Pages>,
    agent_id: &str,
) -> Result<(), String> {
    let label = page_label(agent_id);
    if let Some(w) = app.get_webview(&label) {
        let _ = w.close();
    }
    crate::browserread::forget(&label);
    // The windows its page opened close with it (ESC-695).
    for popup in pages.take_popups(agent_id) {
        if let Some(w) = app.get_webview_window(&popup) {
            let _ = w.destroy();
        }
    }
    pages.remove(agent_id);
    Ok(())
}
