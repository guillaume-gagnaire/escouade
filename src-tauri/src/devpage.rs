//! The page of a launch, inside Escouade (ESC-311). When an agent's test launch answers, its address
//! opens in a webview of its own: in the agent's view (a child of the main window, placed over the
//! spot the window keeps for it), or in the project's « Pages » window, one tab per worktree. One
//! page per agent: launching again reloads the same page.
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
//! - its devtools are its own (`open_devtools` on its webview); the window's stay off in a release.

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, Url, WebviewUrl};

/// The labels of the pages' webviews: `devpage-<agent>`.
pub const PAGE_PREFIX: &str = "devpage-";
/// The labels of the « Pages » windows: `pages-<project>`.
pub const WINDOW_PREFIX: &str = "pages-";
/// The labels of the bars of the « Pages » windows (Escouade's own interface): `pagesbar-<project>`.
pub const BAR_PREFIX: &str = "pagesbar-";
/// The height of the bar of a « Pages » window: its tabs and the page's bar, in logical pixels.
pub const BAR_HEIGHT: f64 = 84.0;
/// The address a page goes to, by its shortcut (F12, Cmd+Opt+I, Ctrl+Shift+I), to ask for its
/// devtools: refused, and the devtools open (`judge` → `Verdict::Devtools`).
pub const DEVTOOLS_URL: &str = "escouade-devtools://open/";
/// The address a page goes to, by its shortcut (Ctrl+Shift+M, Cmd+Shift+M), to be shown at the
/// next width (`Verdict::Responsive`).
pub const RESPONSIVE_URL: &str = "escouade-devtools://responsive/";

/// The width a page is shown at: the whole room, a tablet's, a phone's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageMode {
    #[default]
    Full,
    Tablet,
    Mobile,
}

impl PageMode {
    /// Its width in logical pixels: an iPad in portrait, a recent iPhone; none for the whole room.
    pub fn device_width(self) -> Option<f64> {
        match self {
            PageMode::Full => None,
            PageMode::Tablet => Some(768.0),
            PageMode::Mobile => Some(390.0),
        }
    }
}

/// Where a page shown at `mode` goes in a room `width` wide: its width and its left, centred;
/// narrowed to the room when the room is narrower than the device.
pub fn device_fit(mode: PageMode, width: f64) -> (f64, f64) {
    match mode.device_width() {
        None => (width, 0.0),
        Some(device) => {
            let w = device.min(width).max(1.0);
            (w, ((width - w) / 2.0).round().max(0.0))
        }
    }
}

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

/// The label of the « Pages » window of project `project_id`.
pub fn window_label(project_id: &str) -> String {
    format!("{WINDOW_PREFIX}{}", clean(project_id))
}

/// The label of the bar of the « Pages » window of project `project_id`.
pub fn bar_label(project_id: &str) -> String {
    format!("{BAR_PREFIX}{}", clean(project_id))
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
    /// Its shortcut for its devtools: refused, and they open.
    Devtools,
    /// Its shortcut for the next width: refused, and the window shows it so.
    Responsive,
    /// Escouade's own pages, a file, a script, another scheme: refused, opened nowhere.
    Nowhere,
}

/// Where `url`, asked by the page of the launch at `home`, leads. `own` says whether an address is
/// Escouade's own (`navigation::destination`): never loaded in a page, even when it is on the
/// launch's origin (the dev server's, when the launch is Escouade itself).
pub fn judge(url: &Url, home: &Url, own: impl Fn(&Url) -> bool) -> Verdict {
    if url.as_str() == DEVTOOLS_URL {
        return Verdict::Devtools;
    }
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

/// What the page's shortcuts run in it: F12, Cmd+Opt+I (macOS) and Ctrl+Shift+I ask for its
/// devtools by `DEVTOOLS_URL`; Ctrl+Shift+M (Cmd+Shift+M) for the next width by `RESPONSIVE_URL`.
/// Nothing else is added to the page.
pub fn shortcut_script() -> String {
    format!(
        "window.addEventListener('keydown', function (e) {{\
           var k = (e.key || '').toLowerCase();\
           var i = k === 'i' || e.code === 'KeyI', m = k === 'm' || e.code === 'KeyM';\
           if (e.key === 'F12' || (i && ((e.metaKey && e.altKey) || (e.ctrlKey && e.shiftKey)))) {{\
             e.preventDefault(); e.stopPropagation(); window.location.href = '{DEVTOOLS_URL}';\
           }} else if (m && e.shiftKey && !e.altKey && (e.ctrlKey || e.metaKey)) {{\
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
    /// In the project's « Pages » window, not in the agent's view.
    pub detached: bool,
    /// The width it is shown at.
    pub mode: PageMode,
}

/// What happened to a page, for the window (`UiEvent::DevPage`) and the « Pages » windows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageEvent {
    pub agent_id: String,
    /// `started` and `finished` (a load, at `url`), `outside` (`url` opened in the system's
    /// browser), `refused` (`url` opened nowhere), `devtools`, `attached`, `detached`, `closed`.
    pub phase: &'static str,
    pub url: String,
}

/// Tells a page's events.
pub type Tell = Arc<dyn Fn(PageEvent) + Send + Sync>;

/// The pages open, by agent.
#[derive(Default)]
pub struct Pages {
    pages: Mutex<HashMap<String, PageView>>,
    /// The page each « Pages » window shows, by project.
    shown: Mutex<HashMap<String, String>>,
}

impl Pages {
    pub fn get(&self, agent_id: &str) -> Option<PageView> {
        self.pages.lock().get(agent_id).cloned()
    }

    /// The pages of project `project_id` (its « Pages » window's tabs, detached or not).
    pub fn of_project(&self, project_id: &str) -> Vec<PageView> {
        let mut pages: Vec<_> = self
            .pages
            .lock()
            .values()
            .filter(|p| p.project_id == project_id)
            .cloned()
            .collect();
        pages.sort_by(|a, b| a.title.cmp(&b.title));
        pages
    }

    pub fn all(&self) -> Vec<PageView> {
        self.pages.lock().values().cloned().collect()
    }

    fn put(&self, page: PageView) {
        self.pages.lock().insert(page.agent_id.clone(), page);
    }

    fn set_url(&self, agent_id: &str, url: &str) {
        if let Some(p) = self.pages.lock().get_mut(agent_id) {
            p.url = url.to_string();
        }
    }

    fn set_detached(&self, agent_id: &str, detached: bool) -> Option<PageView> {
        let mut pages = self.pages.lock();
        let p = pages.get_mut(agent_id)?;
        p.detached = detached;
        Some(p.clone())
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
        self.pages.lock().remove(agent_id)
    }

    pub fn shown_in_window(&self, project_id: &str) -> Option<String> {
        self.shown.lock().get(project_id).cloned()
    }

    fn show_in_window(&self, project_id: &str, agent_id: Option<&str>) {
        let mut shown = self.shown.lock();
        match agent_id {
            Some(a) => shown.insert(project_id.to_string(), a.to_string()),
            None => shown.remove(project_id),
        };
    }
}

/// What opening a page needs.
pub struct Open<'a> {
    pub agent_id: &'a str,
    pub project_id: &'a str,
    pub title: &'a str,
    pub url: &'a str,
    /// In the « Pages » window of its project, rather than in the agent's view.
    pub detached: bool,
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
        if let Some(p) = pages.get(o.agent_id) {
            if p.detached {
                show_in_window(app, pages, &p.project_id, o.agent_id)?;
            }
        }
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
        detached: o.detached,
        mode: PageMode::Full,
    };
    if o.detached {
        let window = pages_window(app, pages, o.project_id)?;
        let size = window.inner_size().map_err(|e| e.to_string())?;
        let scale = window.scale_factor().unwrap_or(1.0);
        let (w, h) = (size.width as f64 / scale, size.height as f64 / scale);
        window
            .add_child(
                builder,
                LogicalPosition::new(0.0, BAR_HEIGHT),
                LogicalSize::new(w, (h - BAR_HEIGHT).max(0.0)),
            )
            .map_err(|e| e.to_string())?;
        pages.put(page);
        show_in_window(app, pages, o.project_id, o.agent_id)?;
        return Ok(());
    }
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
    b = with_store(b, data_root, project_id);
    // Every address it is asked to load.
    {
        let (app, home, own, tell) = (app.clone(), home.clone(), own.clone(), tell.clone());
        let (agent, label) = (agent_id.to_string(), label.to_string());
        b = b.on_navigation(move |to| {
            let verdict = judge(to, &home, |u| own(u));
            if verdict != Verdict::Here {
                act(&app, &label, &agent, to, verdict, &tell);
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
                verdict => act(&app, &label, &agent, &to, verdict, &tell),
            }
            NewWindowResponse::Deny
        });
    }
    // Its loads, for its bar.
    {
        let (app, pages, tell) = (app.clone(), pages.clone(), tell.clone());
        let (agent, project) = (agent_id.to_string(), project_id.to_string());
        b = b.on_page_load(move |_, load| {
            let url = load.url().to_string();
            pages.set_url(&agent, &url);
            if pages.get(&agent).is_some_and(|p| p.detached) {
                tell_bar(&app, &project, &pages);
            }
            tell(PageEvent {
                agent_id: agent.clone(),
                phase: match load.event() {
                    PageLoadEvent::Started => "started",
                    PageLoadEvent::Finished => "finished",
                },
                url,
            });
        });
    }
    b
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
fn act<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
    agent: &str,
    to: &Url,
    verdict: Verdict,
    tell: &Tell,
) {
    let phase = match verdict {
        // The window opens it, as its own links (`open_url`).
        Verdict::Outside => "outside",
        Verdict::Devtools => {
            if let Some(w) = app.get_webview(label) {
                w.open_devtools();
            }
            "devtools"
        }
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
        url: if matches!(verdict, Verdict::Devtools | Verdict::Responsive) {
            String::new()
        } else {
            to.to_string()
        },
    });
}

/// The « Pages » window of project `project_id`, made with its bar when it is not there yet.
fn pages_window<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Arc<Pages>,
    project_id: &str,
) -> Result<tauri::Window<R>, String> {
    let label = window_label(project_id);
    if let Some(w) = app.get_window(&label) {
        return Ok(w);
    }
    let window = tauri::window::WindowBuilder::new(app, &label)
        .title(window_title(app, project_id))
        .inner_size(1120.0, 760.0)
        .min_inner_size(480.0, 320.0)
        .build()
        .map_err(|e| e.to_string())?;
    let bar = WebviewBuilder::new(
        bar_label(project_id),
        WebviewUrl::App(format!("index.html?pages={}", clean(project_id)).into()),
    )
    .devtools(cfg!(debug_assertions));
    window
        .add_child(
            bar,
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(1120.0, BAR_HEIGHT),
        )
        .map_err(|e| e.to_string())?;
    // Its bar across the top, its page under it, as the window is resized.
    let (app2, pages2, project) = (app.clone(), pages.clone(), project_id.to_string());
    window.on_window_event(move |e| match e {
        tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. } => {
            lay_out(&app2, &pages2, &project);
        }
        tauri::WindowEvent::Destroyed => {
            // Closed: its pages are closed with it.
            for p in pages2.of_project(&project) {
                if p.detached {
                    if let Some(w) = app2.get_webview(&page_label(&p.agent_id)) {
                        let _ = w.close();
                    }
                    pages2.remove(&p.agent_id);
                }
            }
            pages2.show_in_window(&project, None);
        }
        _ => {}
    });
    Ok(window)
}

/// What the title of a « Pages » window says: its project's name and « Pages ».
fn window_title<R: Runtime>(app: &AppHandle<R>, project_id: &str) -> String {
    let name = app
        .try_state::<Arc<crate::core::Core<R>>>()
        .and_then(|core| core.project(project_id).ok().map(|p| p.name))
        .unwrap_or_else(|| project_id.to_string());
    format!("{name} · Pages")
}

/// The bar and the page shown of the « Pages » window of project `project_id`, at its size.
fn lay_out<R: Runtime>(app: &AppHandle<R>, pages: &Pages, project_id: &str) {
    let Some(window) = app.get_window(&window_label(project_id)) else {
        return;
    };
    let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) else {
        return;
    };
    let (w, h) = (size.width as f64 / scale, size.height as f64 / scale);
    if let Some(bar) = app.get_webview(&bar_label(project_id)) {
        let _ = bar.set_position(LogicalPosition::new(0.0, 0.0));
        let _ = bar.set_size(LogicalSize::new(w, BAR_HEIGHT));
    }
    let shown = pages.shown_in_window(project_id);
    for p in pages
        .of_project(project_id)
        .into_iter()
        .filter(|p| p.detached)
    {
        let Some(page) = app.get_webview(&page_label(&p.agent_id)) else {
            continue;
        };
        if shown.as_deref() == Some(p.agent_id.as_str()) {
            let (pw, left) = device_fit(p.mode, w);
            let _ = page.set_position(LogicalPosition::new(left, BAR_HEIGHT));
            let _ = page.set_size(LogicalSize::new(pw, (h - BAR_HEIGHT).max(1.0)));
            let _ = page.show();
        } else {
            let _ = page.hide();
        }
    }
}

/// The « Pages » window of the project shows the page of agent `agent_id`, and comes to the front.
pub fn show_in_window<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Arc<Pages>,
    project_id: &str,
    agent_id: &str,
) -> Result<(), String> {
    pages.show_in_window(project_id, Some(agent_id));
    lay_out(app, pages, project_id);
    if let Some(w) = app.get_window(&window_label(project_id)) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    tell_bar(app, project_id, pages);
    Ok(())
}

/// The pages of a « Pages » window and the one it shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagesOf {
    pub pages: Vec<PageView>,
    pub shown: Option<String>,
}

/// The bar of the « Pages » window of the project reads its pages again.
pub fn tell_bar<R: Runtime>(app: &AppHandle<R>, project_id: &str, pages: &Pages) {
    use tauri::Emitter;
    let _ = app.emit_to(
        tauri::EventTarget::webview(bar_label(project_id)),
        "devpages",
        PagesOf {
            pages: pages.of_project(project_id),
            shown: pages.shown_in_window(project_id),
        },
    );
}

/// The page of agent `agent_id` in the agent's view: placed at `bounds` and shown, or hidden.
pub fn place<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Pages,
    agent_id: &str,
    bounds: Option<Bounds>,
) -> Result<(), String> {
    let Some(page) = pages.get(agent_id) else {
        return Ok(());
    };
    if page.detached {
        return Ok(());
    }
    let Some(webview) = app.get_webview(&page_label(agent_id)) else {
        return Ok(());
    };
    match bounds {
        Some(b) if b.width >= 1.0 && b.height >= 1.0 => {
            webview
                .set_position(LogicalPosition::new(b.x, b.y))
                .map_err(|e| e.to_string())?;
            webview
                .set_size(LogicalSize::new(b.width, b.height))
                .map_err(|e| e.to_string())?;
            webview.show().map_err(|e| e.to_string())
        }
        _ => webview.hide().map_err(|e| e.to_string()),
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

/// Shows the page of agent `agent_id` at `mode`: in the « Pages » window, laid out again at once
/// (in the agent's view, the window places it by its mode).
pub fn set_mode<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Pages,
    agent_id: &str,
    mode: PageMode,
) -> Result<(), String> {
    if !pages.set_mode(agent_id, mode) {
        return Err("no page for this agent".into());
    }
    if let Some(p) = pages.get(agent_id) {
        if p.detached {
            lay_out(app, pages, &p.project_id);
            tell_bar(app, &p.project_id, pages);
        }
    }
    Ok(())
}

/// Opens the devtools of the page of agent `agent_id`, never the window's.
pub fn devtools<R: Runtime>(app: &AppHandle<R>, agent_id: &str) -> Result<(), String> {
    let webview = app
        .get_webview(&page_label(agent_id))
        .ok_or("no page for this agent")?;
    webview.open_devtools();
    Ok(())
}

/// The next page the « Pages » window of `project_id` shows, when the one it showed leaves it.
fn next_shown(pages: &Pages, project_id: &str, leaving: &str) {
    if pages.shown_in_window(project_id).as_deref() == Some(leaving) {
        let next = pages
            .of_project(project_id)
            .into_iter()
            .find(|p| p.detached && p.agent_id != leaving)
            .map(|p| p.agent_id);
        pages.show_in_window(project_id, next.as_deref());
    }
}

/// Moves the page of agent `agent_id` to its project's « Pages » window (`detached`), or back to the
/// agent's view (hidden until placed). The page keeps its state, it is not loaded again.
pub fn detach<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Arc<Pages>,
    agent_id: &str,
    detached: bool,
) -> Result<PageView, String> {
    let page = pages.get(agent_id).ok_or("no page for this agent")?;
    let webview = app
        .get_webview(&page_label(agent_id))
        .ok_or("no page for this agent")?;
    if detached {
        let window = pages_window(app, pages, &page.project_id)?;
        webview.reparent(&window).map_err(|e| e.to_string())?;
        let page = pages.set_detached(agent_id, true).ok_or("gone")?;
        show_in_window(app, pages, &page.project_id, agent_id)?;
        return Ok(page);
    }
    let main = app.get_window("main").ok_or("no main window")?;
    let _ = webview.hide();
    webview.reparent(&main).map_err(|e| e.to_string())?;
    next_shown(pages, &page.project_id, agent_id);
    let page = pages.set_detached(agent_id, false).ok_or("gone")?;
    lay_out(app, pages, &page.project_id);
    tell_bar(app, &page.project_id, pages);
    // The window has no tab left: it closes.
    if !pages
        .of_project(&page.project_id)
        .iter()
        .any(|p| p.detached)
    {
        if let Some(w) = app.get_window(&window_label(&page.project_id)) {
            let _ = w.close();
        }
    }
    Ok(page)
}

/// Closes the page of agent `agent_id` (its agent gone, its ticket done).
pub fn close<R: Runtime>(
    app: &AppHandle<R>,
    pages: &Arc<Pages>,
    agent_id: &str,
) -> Result<(), String> {
    if let Some(w) = app.get_webview(&page_label(agent_id)) {
        let _ = w.close();
    }
    if let Some(page) = pages.get(agent_id) {
        if page.detached {
            next_shown(pages, &page.project_id, agent_id);
        }
        pages.remove(agent_id);
        if page.detached {
            lay_out(app, pages, &page.project_id);
            tell_bar(app, &page.project_id, pages);
        }
    }
    Ok(())
}
