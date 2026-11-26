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
//! - it stays on its launch's origin (`judge`): Escouade's own addresses are refused, another
//!   origin opens in the system's browser, anything else nowhere;
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
use tauri::webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, Url, WebviewUrl};

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
    /// Its launch's origin (or a blank page): the page loads it.
    Here,
    /// Another web or mail address: refused, the system's browser or mail client opens it.
    Outside,
    /// Its shortcut for the next width: refused, and the window shows it so.
    Responsive,
    /// Escouade's own pages, a file, a script, another scheme: refused, opened nowhere.
    Nowhere,
}

/// Where `url`, asked by the page of the launch at `home`, leads. `own` says whether an address is
/// Escouade's own (`navigation::destination`): never loaded in a page, even when it is on the
/// launch's origin (the dev server's, when the launch is Escouade itself).
pub fn judge(url: &Url, home: &Url, own: impl Fn(&Url) -> bool) -> Verdict {
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
        "http" | "https" if url.origin() == home.origin() => Verdict::Here,
        "http" | "https" | "mailto" => Verdict::Outside,
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
    /// The order it opened in, the oldest first: its row's place in « Navigateurs ».
    #[serde(skip)]
    pub opened: u64,
}

/// What happened to a page, for the window (`UiEvent::DevPage`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageEvent {
    pub agent_id: String,
    /// `started` and `finished` (a load, at `url`), `outside` (`url` opened in the system's
    /// browser), `refused` (`url` opened nowhere), `responsive` (its width's shortcut), `title`
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
    /// The order the pages opened in.
    seq: std::sync::atomic::AtomicU64,
    /// The captures each agent took in the last minute (`capture_browser`, ESC-670), by agent.
    shots: Mutex<HashMap<String, std::collections::VecDeque<std::time::Instant>>>,
}

/// What the agents may read of one page outside the launch.
#[derive(Default)]
struct Leaves {
    asks: Vec<Consent>,
    grants: Vec<Consent>,
    refused: Vec<Consent>,
}

impl Pages {
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
        // Its leaves fall with it (ESC-669).
        self.consents.lock().remove(agent_id);
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
}

/// Whether an address is Escouade's own, for the main window's setup.
pub type Own = Arc<dyn Fn(&Url) -> bool + Send + Sync>;

/// Opens the page of agent `o.agent_id` at `o.url`, or, when it is open, loads `o.url` in it
/// (launching again reloads the same page).
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
    crate::browserread::attach(&webview, pages.new_logs(o.agent_id));
    pages.put(page);
    Ok(())
}

/// The webview of a page: its launch's origin alone, its own storage, no command, its shortcut.
#[allow(clippy::too_many_arguments)]
fn builder<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Arc<Pages>,
    label: &str,
    agent_id: &str,
    project_id: &str,
    url: Url,
    data_root: &Path,
    own: Own,
    tell: Tell,
) -> WebviewBuilder<R> {
    let home = url.clone();
    let mut b = WebviewBuilder::new(label, WebviewUrl::External(url))
        .devtools(true)
        .initialization_script(shortcut_script());
    // WebKit gives the app no console: a relay in the page tells Escouade's world (ESC-668).
    if cfg!(not(windows)) {
        b = b.initialization_script(crate::browserread::RELAY);
    }
    b = with_store(b, data_root, project_id);
    // Every address it is asked to load.
    {
        let (home, own, tell) = (home.clone(), own.clone(), tell.clone());
        let agent = agent_id.to_string();
        b = b.on_navigation(move |to| {
            let verdict = judge(to, &home, |u| own(u));
            if verdict != Verdict::Here {
                act(&agent, to, verdict, &tell);
            }
            verdict == Verdict::Here
        });
    }
    // Its new windows: on its origin, in the same page (one page per worktree); else as above.
    {
        let (app, home, own, tell) = (app.clone(), home.clone(), own.clone(), tell.clone());
        let (agent, label) = (agent_id.to_string(), label.to_string());
        b = b.on_new_window(move |to, _| {
            match judge(&to, &home, |u| own(u)) {
                Verdict::Here => {
                    if let Some(w) = app.get_webview(&label) {
                        let _ = w.navigate(to);
                    }
                }
                verdict => act(&agent, &to, verdict, &tell),
            }
            NewWindowResponse::Deny
        });
    }
    // Its loads, for its bar.
    {
        let (pages, tell) = (pages.clone(), tell.clone());
        let agent = agent_id.to_string();
        b = b.on_page_load(move |_, load| {
            let url = load.url().to_string();
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

/// Back, forward, reload, or an address typed in its bar (judged by the page's guard like any
/// other: another origin opens outside).
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
    pages.remove(agent_id);
    Ok(())
}
