//! The pages of the agents (docs/AGENT-PAGES.md, AP-1, ESC-667): the shelter they are shown in.
//! An agent's page is HTML an agent wrote, maybe after reading something hostile: Escouade never
//! trusts it. Here is only the shelter (§ 8); the MCP tool, the view and the way back come after
//! (AP-2 to AP-4).
//!
//! - **A webview apart**, labelled `agentpage-<p-n>`, a child of the main window as a browser is.
//!   No command answers it (`refuses`, read by the window's thread wrapper), and the capability
//!   names the window's webviews alone: no plugin answers it either.
//! - **An origin of its own**: the scheme `escouade-page://`, a host per page
//!   (`escouade-page://p-12.localhost/`; WebView2 serves it as `http://escouade-page.p-12.localhost/`,
//!   still one origin per page). The handler serves a page **to its own webview alone**, told by the
//!   label of the webview asking (`UriSchemeContext::webview_label`): p-12 never reads p-13, and the
//!   window never reads a page.
//! - **Nothing but its copy**: the files of its folder (the version Escouade copied, AP-2), by a
//!   cleaned relative path; no `..`, no link, never a worktree.
//! - **Its CSP**, in a header of every answer and in a `<meta>` put at the head of its document: no
//!   network (`connect-src`, `img-src`, `media-src`… on its own origin alone), no frame, no worker,
//!   no form, no `unsafe-eval` (P4: scripts allowed, never code made from text).
//! - **No navigation** off the page (`judge`): another address opens nowhere; a web address is kept
//!   for the window to offer (« La page veut ouvrir… »), opened outside by a gesture of the user
//!   alone (`outside`). New windows are always denied.
//! - **No storage that lasts**: `incognito` (InPrivate, a non-persistent store, an ephemeral
//!   context). **No permission** (`on_permission_request`) and **no download** (`on_download`).
//! - **What the CSP does not cover**: the prefetches taken out of what is served (and
//!   `X-DNS-Prefetch-Control: off`), WebRTC taken away by a script of Escouade before the page's,
//!   in every frame.
//!
//! What a real webview does with it on each system is said by `agentpage_probe` (examples; in the
//! CI under Linux and macOS) and by `tests/e2e/agentpage.spec.ts` (Windows), with the same trap
//! (`agentpage_trap.html`).

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tauri::http::{Response, StatusCode};
use tauri::webview::{NewWindowResponse, PageLoadEvent, PermissionResponse, WebviewBuilder};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, Url, WebviewUrl};

pub use crate::devpage::Bounds;

/// The labels of the agents' pages' webviews: `agentpage-<p-n>`.
pub const PREFIX: &str = "agentpage-";
/// The scheme Escouade serves the pages by.
pub const SCHEME: &str = "escouade-page";
/// The trap of `agentpage_probe` and of the end-to-end test: it tries every way out.
pub const TRAP: &str = include_str!("agentpage_trap.html");

/// Whether `id` is a page's number: `p-` and 1 to 9 digits, without a leading zero.
pub fn is_page(id: &str) -> bool {
    id.strip_prefix("p-").is_some_and(|n| {
        !n.is_empty()
            && n.len() <= 9
            && !n.starts_with('0')
            && n.bytes().all(|b| b.is_ascii_digit())
    })
}

/// The label of page `page`'s webview.
pub fn label(page: &str) -> String {
    format!("{PREFIX}{page}")
}

/// The page whose webview is `label`, if it is one.
pub fn page_of(label: &str) -> Option<&str> {
    label.strip_prefix(PREFIX).filter(|p| is_page(p))
}

/// A call from the webview `label` that no command answers: one of an agent's page.
pub fn refuses(label: &str) -> bool {
    label.starts_with(PREFIX)
}

/// The host of page `page` under the scheme.
fn host(page: &str) -> String {
    format!("{page}.localhost")
}

/// The address page `page` opens at, as Escouade asks for it (WebView2 turns it into
/// `http://escouade-page.<host>/`).
pub fn url(page: &str) -> Url {
    Url::parse(&format!("{SCHEME}://{}/", host(page))).expect("a page's address")
}

/// The address a webview is told to go to for page `page`: WebView2 is told it in its `http` form
/// (`navigate` does not turn the scheme into it, as the first load does).
pub fn address(page: &str) -> Url {
    if cfg!(windows) {
        Url::parse(&format!("{}/", origin(page, true))).expect("a page's address")
    } else {
        url(page)
    }
}

/// The origin of page `page`, as its document sees it: what its CSP allows, alone.
pub fn origin(page: &str, windows: bool) -> String {
    if windows {
        format!("http://{SCHEME}.{}", host(page))
    } else {
        format!("{SCHEME}://{}", host(page))
    }
}

/// The CSP of page `page`. Its own origin, written out rather than `'self'`: on Windows `'self'`
/// would also let a WebSocket reach `ws://escouade-page.p-12.localhost`, the loopback's port 80.
pub fn csp(page: &str, windows: bool) -> String {
    let me = origin(page, windows);
    format!(
        "default-src 'none'; script-src {me} 'unsafe-inline'; style-src {me} 'unsafe-inline'; \
         img-src {me} data: blob:; font-src {me} data:; media-src {me} blob:; connect-src {me}; \
         frame-src 'none'; child-src 'none'; worker-src 'none'; object-src 'none'; \
         form-action 'none'; base-uri 'none'; manifest-src 'none'"
    )
}

/// What Escouade runs in every frame of a page before the page's own scripts (`agentpage_guard.js`):
/// WebRTC taken away, code made from text refused, prefetches a script adds tamed. Nothing else.
pub fn init_script() -> &'static str {
    include_str!("agentpage_guard.js")
}

/// How a page's document is hardened: both, always, but by the probe, which takes one away to see
/// whether a webview honours the other alone (`agentpage_probe`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Harden {
    /// The CSP in a header of each answer.
    pub header: bool,
    /// The CSP in a `<meta>` at the head of each HTML document.
    pub meta: bool,
}

impl Default for Harden {
    fn default() -> Self {
        Harden {
            header: true,
            meta: true,
        }
    }
}

/// The HTML Escouade serves of `html`: the CSP of page `page` in a `<meta>` first (after the
/// doctype, which must stay first), with `X-DNS-Prefetch-Control: off`; the prefetches taken out.
pub fn harden_html(html: &str, page: &str, windows: bool, meta: bool) -> String {
    let body = strip_prefetches(html);
    if !meta {
        return body;
    }
    let tags = format!(
        "<meta http-equiv=\"Content-Security-Policy\" content=\"{}\">\
         <meta http-equiv=\"x-dns-prefetch-control\" content=\"off\">",
        csp(page, windows)
    );
    let at = doctype_end(&body);
    let mut out = String::with_capacity(body.len() + tags.len());
    out.push_str(&body[..at]);
    out.push_str(&tags);
    out.push_str(&body[at..]);
    out
}

/// Where the doctype ends (after a BOM and blanks), or 0 without one.
fn doctype_end(html: &str) -> usize {
    let start = html.len()
        - html
            .trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n'])
            .len();
    let rest = &html[start..];
    if rest.len() >= 9 && rest[..9].eq_ignore_ascii_case("<!doctype") {
        if let Some(end) = rest.find('>') {
            return start + end + 1;
        }
    }
    0
}

/// `html` without its `<link>` of `dns-prefetch`, `preconnect`, `prefetch` or `prerender`, nor
/// its rules of speculation: they leak before any script, and the CSP does not stop them all.
pub fn strip_prefetches(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while let Some(found) = lower[i..].find('<') {
        let at = i + found;
        out.push_str(&html[i..at]);
        let tag_end = lower[at..]
            .find('>')
            .map(|e| at + e + 1)
            .unwrap_or(html.len());
        let tag = &lower[at..tag_end];
        if is_tag(tag, "link") && leaks(tag) {
            i = tag_end;
            continue;
        }
        if is_tag(tag, "script") && tag.contains("speculationrules") {
            let close = lower[tag_end..]
                .find("</script")
                .map(|c| tag_end + c)
                .and_then(|c| lower[c..].find('>').map(|e| c + e + 1))
                .unwrap_or(html.len());
            i = close;
            continue;
        }
        out.push_str(&html[at..tag_end]);
        i = tag_end;
    }
    out.push_str(&html[i..]);
    out
}

/// Whether the lowercased `tag` opens an element `name`.
fn is_tag(tag: &str, name: &str) -> bool {
    tag.strip_prefix('<')
        .and_then(|t| t.strip_prefix(name))
        .is_some_and(|t| t.starts_with(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>'))
}

/// Whether a lowercased `<link>` asks for a prefetch of some kind.
fn leaks(tag: &str) -> bool {
    ["dns-prefetch", "preconnect", "prefetch", "prerender"]
        .iter()
        .any(|rel| tag.contains(rel))
}

/// A request to the scheme, once its page is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// A file of the page, by its cleaned relative path.
    File(String),
    /// The page's way back (`_/<name>`, AP-4): none answers yet.
    Channel(String),
    /// A file of the templates (`_t/<name>`, AP-5): Escouade's, the same for every page.
    Template(String),
}

/// The page and route a request names: `escouade-page://p-12.localhost/<path>` (WebView2's
/// address comes back in that form too). None for anything else: another host, a port, a user, a
/// path that leaves the page's folder.
pub fn locate(uri: &str) -> Option<(String, Route)> {
    let url = Url::parse(uri).ok()?;
    if url.scheme() != SCHEME || url.port().is_some() || !url.username().is_empty() {
        return None;
    }
    let page = url.host_str()?.strip_suffix(".localhost")?;
    if !is_page(page) {
        return None;
    }
    // The raw path, before the URL parser folded its `..`: one in it is refused, not followed.
    let raw = uri.split_once("://")?.1;
    let raw = raw[raw.find('/').unwrap_or(raw.len())..]
        .split(['?', '#'])
        .next()
        .unwrap_or("");
    let rel = clean_path(raw)?;
    let route = if let Some(name) = rel.strip_prefix("_/") {
        Route::Channel(name.to_string())
    } else if rel == "_" {
        Route::Channel(String::new())
    } else if let Some(name) = rel.strip_prefix("_t/") {
        Route::Template(name.to_string())
    } else {
        Route::File(rel)
    };
    Some((page.to_string(), route))
}

/// A path of the scheme as a relative path of the page's folder: decoded, `index.html` for a
/// folder; none with an empty, `.` or `..` segment, a backslash, a colon or a control character.
fn clean_path(raw: &str) -> Option<String> {
    let decoded = decode(raw)?;
    let mut parts = Vec::new();
    let trimmed = decoded.strip_prefix('/').unwrap_or(&decoded);
    let folder = trimmed.is_empty() || trimmed.ends_with('/');
    for part in trimmed.split('/') {
        if part.is_empty() {
            continue;
        }
        if part == "."
            || part == ".."
            || part.contains(['\\', ':'])
            || part.chars().any(|c| c.is_control())
        {
            return None;
        }
        parts.push(part);
    }
    if decoded.contains("//") {
        return None;
    }
    if folder {
        parts.push("index.html");
    }
    Some(parts.join("/"))
}

/// `%XX` decoded, as UTF-8; none for a broken escape or bytes that are not UTF-8.
pub(crate) fn decode(s: &str) -> Option<String> {
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

/// The type of a file Escouade serves, by its extension; anything else is bytes (`nosniff`: never
/// read as something else).
pub fn content_type(rel: &str) -> &'static str {
    let ext = rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        _ => "application/octet-stream",
    }
}

/// The file `rel` of the folder `root`, read: none when it is not a plain file under `root`, or
/// when any part of its way there is a link.
fn read_file(root: &Path, rel: &str) -> Option<Vec<u8>> {
    let mut at = root.to_path_buf();
    if std::fs::symlink_metadata(&at)
        .ok()?
        .file_type()
        .is_symlink()
    {
        return None;
    }
    for part in Path::new(rel).components() {
        let Component::Normal(name) = part else {
            return None;
        };
        at.push(name);
        if std::fs::symlink_metadata(&at)
            .ok()?
            .file_type()
            .is_symlink()
        {
            return None;
        }
    }
    let real = at.canonicalize().ok()?;
    if !real.starts_with(root.canonicalize().ok()?) || !real.is_file() {
        return None;
    }
    std::fs::read(real).ok()
}

/// A page, as the shelter keeps it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageView {
    pub page: String,
    /// Where it is now.
    pub url: String,
    /// Its last load finished.
    pub loaded: bool,
    /// Its document's title, as the page wrote it (16 000 characters at most; the window cleans it
    /// before it shows it).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// The web address it last asked to open, kept for the window to offer (« La page veut
    /// ouvrir… »); opened by `outside` alone, on a gesture of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wants: Option<String>,
    /// Downloads it asked for, all refused.
    pub downloads_refused: u32,
    /// Permissions it asked for, all refused.
    pub permissions_refused: u32,
    /// Addresses it tried to go to, all refused.
    pub navigations_refused: u32,
}

/// What happened to a page, for the window (`UiEvent::AgentPage`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageEvent {
    pub page: String,
    /// `started` and `finished` (a load, at `url`), `wants` (it asked for the web address `url`:
    /// the window offers it), `refused` (another address, opened nowhere), `download` and
    /// `permission` (refused), `title` (its document's title, in `title`).
    pub phase: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
}

/// Tells a page's events.
pub type Tell = Arc<dyn Fn(PageEvent) + Send + Sync>;

/// What the shelter serves of a page.
struct Served {
    root: PathBuf,
    harden: Harden,
}

/// The agents' pages: what the scheme serves, by page, and the pages open.
#[derive(Default)]
pub struct Shelter {
    served: Mutex<HashMap<String, Served>>,
    views: Mutex<HashMap<String, PageView>>,
    /// The pages whose webview has no filter yet (Windows): served nothing.
    unfiltered: Mutex<std::collections::HashSet<String>>,
    /// The data folder of each open page's webview (Linux), new at each opening, gone at its close.
    stores: Mutex<HashMap<String, PathBuf>>,
}

impl Shelter {
    /// Serves the folder `root` as page `page` (to its webview alone).
    pub fn put(&self, page: &str, root: PathBuf) -> Result<(), String> {
        self.put_hardened(page, root, Harden::default())
    }

    /// As `put`, hardened as `harden` says: the probe's alone.
    pub fn put_hardened(&self, page: &str, root: PathBuf, harden: Harden) -> Result<(), String> {
        if !is_page(page) {
            return Err(format!("not a page: {page}"));
        }
        self.served
            .lock()
            .insert(page.to_string(), Served { root, harden });
        Ok(())
    }

    /// Serves page `page` no more (its purge, AP-2).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn remove(&self, page: &str) {
        self.served.lock().remove(page);
    }

    pub fn view(&self, page: &str) -> Option<PageView> {
        self.views.lock().get(page).cloned()
    }

    fn with_view(&self, page: &str, f: impl FnOnce(&mut PageView)) {
        if let Some(v) = self.views.lock().get_mut(page) {
            f(v);
        }
    }

    /// The answer to a request of the webview `label` for `uri`. A page is served to its own
    /// webview alone; anything else is a 404 that says nothing (another page, the window, a path
    /// out of the folder, a method but GET and HEAD, a channel: none is open in AP-1).
    pub fn serve(&self, label: &str, method: &str, uri: &str, windows: bool) -> Response<Vec<u8>> {
        let nothing = || {
            answer(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                b"".to_vec(),
                None,
            )
        };
        let Some((page, route)) = locate(uri) else {
            return nothing();
        };
        if page_of(label) != Some(page.as_str()) {
            log::warn!("the webview {label} asked for page {page}: refused");
            return nothing();
        }
        if !matches!(method, "GET" | "HEAD") {
            return nothing();
        }
        let rel = match route {
            Route::File(rel) => rel,
            Route::Template(name) => {
                if self.unfiltered.lock().contains(&page) {
                    return nothing();
                }
                let Some(header) = self.served.lock().get(&page).map(|s| s.harden.header) else {
                    return nothing();
                };
                let Some((kind, bytes)) = crate::agentpage_templates::asset(&name) else {
                    return nothing();
                };
                let body = if method == "HEAD" {
                    Vec::new()
                } else {
                    bytes.into_owned()
                };
                return answer(
                    StatusCode::OK,
                    kind,
                    body,
                    header.then(|| csp(&page, windows)),
                );
            }
            Route::Channel(_) => return nothing(),
        };
        if self.unfiltered.lock().contains(&page) {
            return nothing();
        }
        let (root, harden) = match self.served.lock().get(&page) {
            Some(s) => (s.root.clone(), s.harden),
            None => return nothing(),
        };
        let Some(bytes) = read_file(&root, &rel) else {
            return nothing();
        };
        let kind = content_type(&rel);
        let bytes = if kind.starts_with("text/html") {
            harden_html(
                &String::from_utf8_lossy(&bytes),
                &page,
                windows,
                harden.meta,
            )
            .into_bytes()
        } else {
            bytes
        };
        let policy = harden.header.then(|| csp(&page, windows));
        let body = if method == "HEAD" { Vec::new() } else { bytes };
        answer(StatusCode::OK, kind, body, policy)
    }
}

/// An answer of the scheme, with the headers every one has.
fn answer(status: StatusCode, kind: &str, body: Vec<u8>, csp: Option<String>) -> Response<Vec<u8>> {
    let mut b = Response::builder()
        .status(status)
        .header("Content-Type", kind)
        .header("X-Content-Type-Options", "nosniff")
        .header("X-DNS-Prefetch-Control", "off")
        .header("Cache-Control", "no-store")
        .header("Referrer-Policy", "no-referrer")
        .header(
            "Permissions-Policy",
            "camera=(), microphone=(), geolocation=(), clipboard-read=(), clipboard-write=(), \
             display-capture=(), usb=(), serial=(), hid=(), midi=(), payment=(), \
             screen-wake-lock=(), xr-spatial-tracking=()",
        );
    if let Some(policy) = csp {
        b = b.header("Content-Security-Policy", policy);
    }
    b.body(body).unwrap_or_else(|_| Response::new(Vec::new()))
}

/// What an address a page asks to load leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Its own page: it loads.
    Here,
    /// A web or mail address: refused; kept for the window to offer, opened on a gesture alone.
    Outside,
    /// Anything else (another page, Escouade, a file, a script): refused, opened nowhere.
    Nowhere,
}

/// Where `to`, asked by page `page`, leads. WebView2 tells the page's own address in its `http`
/// form, which is the page's on Windows alone: elsewhere it is the loopback's port 80. `own` says
/// whether an address is Escouade's own (`navigation::destination`): never offered.
pub fn judge(to: &Url, page: &str, windows: bool, own: impl Fn(&Url) -> bool) -> Verdict {
    let mine = match to.scheme() {
        s if s == SCHEME => to.host_str() == Some(host(page).as_str()) && to.port().is_none(),
        "http" if windows => {
            to.host_str() == Some(format!("{SCHEME}.{}", host(page)).as_str())
                && to.port().is_none()
        }
        _ => false,
    };
    if mine && to.username().is_empty() {
        return Verdict::Here;
    }
    let other_page = to.host_str().is_some_and(|h| {
        h.ends_with(".localhost") && (to.scheme() == SCHEME || h.starts_with(SCHEME))
    });
    match to.scheme() {
        "http" | "https" | "mailto" if !other_page && !own(to) => Verdict::Outside,
        _ => Verdict::Nowhere,
    }
}

/// What opening a page needs.
pub struct Open<'a> {
    pub page: &'a str,
    /// Where it goes in the main window (hidden until placed, without).
    pub bounds: Option<Bounds>,
    pub visible: bool,
    /// Escouade's data folder: WebView2's browser of the pages keeps its own folder there.
    pub data_root: &'a Path,
}

/// How many times a page reached for the network and was cut, by the filter of its WebView2 or
/// by the dead end.
static CUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Counts a reach for the network that was cut, and says so in the log (the first ones).
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn cut_one() {
    let n = CUT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    if n <= 20 || n.is_power_of_two() {
        log::warn!("an agent page reached for the network: cut ({n})");
    }
}

/// How many times a page reached for the network and was cut.
pub fn cut() -> u64 {
    CUT.load(std::sync::atomic::Ordering::Relaxed)
}

/// A proxy of Escouade's own on the loopback that lets nothing through (Windows, Linux): every
/// connection it is given is dropped at once. The pages' webviews go to the network through it
/// alone, so that what neither the CSP, `on_navigation` nor WebView2's filter stops reaches
/// nothing: the probe saw WebView2 connect to the address of a navigation it had refused (a
/// preconnection, after a DNS lookup of its host). Its port, the same for the whole run (WebView2's
/// browser of the pages starts once with it); none when it could not listen.
#[cfg(not(target_os = "macos"))]
pub fn dead_end() -> Option<u16> {
    static PORT: std::sync::OnceLock<Option<u16>> = std::sync::OnceLock::new();
    *PORT.get_or_init(|| {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
        let port = listener.local_addr().ok()?.port();
        std::thread::Builder::new()
            .name("agentpage-dead-end".into())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    cut_one();
                    drop(stream);
                }
            })
            .ok()?;
        Some(port)
    })
}

/// WebView2's browser of the pages: wry's own switches, its network through the dead end alone
/// (the loopback too: `<-loopback>` takes away the bypass Chromium gives it), WebRTC never
/// beside a proxy, no DNS prefetch. Other switches than the window's need a folder of their own.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn browser_args(port: u16) -> String {
    format!(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection          --proxy-server=http://127.0.0.1:{port} --proxy-bypass-list=<-loopback>          --force-webrtc-ip-handling-policy=disable_non_proxied_udp --dns-prefetch-disable"
    )
}

/// Whether a request WebView2 makes for page `page` is its own (or a local `data:`, `blob:`,
/// `about:`): every other is answered 403 by Escouade before it reaches the network.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn own_request(uri: &str, page: &str) -> bool {
    let mine = format!("http://{SCHEME}.{}/", host(page));
    uri.starts_with(&mine)
        || ["data:", "blob:", "about:"]
            .iter()
            .any(|s| uri.starts_with(s))
}

/// Windows: the filter of a page's WebView2. Every request it makes, of any frame, worker or
/// kind, passes by WebView2's `WebResourceRequested` (the filter `*`): what is not the page's own
/// is answered 403 there, before the network. The probe saw WebView2 send a navigation that
/// `on_navigation` had refused, and a prefetch the CSP let by: neither leaves now.
#[cfg(windows)]
mod filter {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Controller, ICoreWebView2_2, ICoreWebView2_22,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL, COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
    };
    use webview2_com::{take_pwstr, WebResourceRequestedEventHandler};
    use windows_core::{Interface, HSTRING, PWSTR};

    /// # Safety
    /// `controller` must be the live controller of the page's WebView, and this called on its thread.
    pub unsafe fn install(
        controller: &ICoreWebView2Controller,
        page: String,
    ) -> windows_core::Result<()> {
        let core = controller.CoreWebView2()?;
        let all = HSTRING::from("*");
        match core.cast::<ICoreWebView2_22>() {
            Ok(core22) => core22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                &all,
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
            )?,
            Err(_) => {
                core.AddWebResourceRequestedFilter(&all, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)?
            }
        }
        let env = core.cast::<ICoreWebView2_2>()?.Environment()?;
        let handler = WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut uri = PWSTR::null();
            // SAFETY: `args` is the event's, alive for the call; the string is COM's, taken (and
            // so freed) by `take_pwstr`.
            unsafe {
                args.Request()?.Uri(&mut uri)?;
                let uri = take_pwstr(uri);
                if !super::own_request(&uri, &page) {
                    let refused = env.CreateWebResourceResponse(
                        None,
                        403,
                        &HSTRING::from("Forbidden"),
                        &HSTRING::from("Content-Type: text/plain"),
                    )?;
                    args.SetResponse(&refused)?;
                    super::cut_one();
                }
            }
            Ok(())
        }));
        // The registration lasts as long as the WebView: it is never removed.
        let mut token = 0;
        core.add_WebResourceRequested(&handler, &mut token)
    }
}

/// Puts the filter on the WebView2 of page `page` (Windows), and waits for it: the page is not
/// served before. Never called on the window's thread, which runs it.
#[cfg(windows)]
fn install_filter<R: Runtime>(webview: &tauri::Webview<R>, page: &str) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let page = page.to_string();
    webview
        .with_webview(move |platform| {
            // SAFETY: the controller is the live one of the page's WebView, on its thread.
            let done = unsafe { filter::install(&platform.controller(), page) };
            let _ = tx.send(done.map_err(|e| e.to_string()));
        })
        .map_err(|e| e.to_string())?;
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .map_err(|_| "the page's filter was not put in time".to_string())?
}

/// Opens page `o.page` in a webview of its own in the main window: served by the scheme (it must
/// have been `put`), sheltered. Reloads it when it is open. Not on the window's thread.
pub fn open<R: Runtime>(
    app: &AppHandle<R>,
    shelter: &Arc<Shelter>,
    o: Open<'_>,
    own: crate::devpage::Own,
    tell: Tell,
) -> Result<(), String> {
    if !is_page(o.page) {
        return Err(format!("not a page: {}", o.page));
    }
    if !shelter.served.lock().contains_key(o.page) {
        return Err(format!("page {} is not served", o.page));
    }
    let label = label(o.page);
    if let Some(webview) = app.get_webview(&label) {
        webview
            .navigate(address(o.page))
            .map_err(|e| e.to_string())?;
        shelter.with_view(o.page, |v| v.loaded = false);
        return Ok(());
    }
    let window = app.get_window("main").ok_or("no main window")?;
    let b = o.bounds.unwrap_or(Bounds {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    let sheltered = builder(shelter, o.page, o.data_root, own, tell)?;
    shelter.views.lock().insert(
        o.page.to_string(),
        PageView {
            page: o.page.to_string(),
            url: url(o.page).to_string(),
            loaded: false,
            title: String::new(),
            wants: None,
            downloads_refused: 0,
            permissions_refused: 0,
            navigations_refused: 0,
        },
    );
    // Windows: nothing of the page is served until its filter is on.
    #[cfg(windows)]
    shelter.unfiltered.lock().insert(o.page.to_string());
    let webview = window
        .add_child(
            sheltered,
            LogicalPosition::new(b.x, b.y),
            LogicalSize::new(b.width.max(1.0), b.height.max(1.0)),
        )
        .map_err(|e| {
            shelter.views.lock().remove(o.page);
            e.to_string()
        })?;
    #[cfg(windows)]
    {
        if let Err(e) = install_filter(&webview, o.page) {
            let _ = webview.close();
            shelter.views.lock().remove(o.page);
            return Err(e);
        }
        shelter.unfiltered.lock().remove(o.page);
        webview
            .navigate(address(o.page))
            .map_err(|e| e.to_string())?;
    }
    if !(o.visible && o.bounds.is_some()) {
        let _ = webview.hide();
    }
    Ok(())
}

/// The webview of a page, sheltered: its own address alone, incognito, no permission, no
/// download, no new window, WebRTC gone, devtools on a gesture of the user alone (`devtools`).
fn builder<R: Runtime>(
    shelter: &Arc<Shelter>,
    page: &str,
    data_root: &Path,
    own: crate::devpage::Own,
    tell: Tell,
) -> Result<WebviewBuilder<R>, String> {
    let windows = cfg!(windows);
    let mut b = WebviewBuilder::new(label(page), WebviewUrl::CustomProtocol(url(page)))
        .incognito(true)
        .devtools(true)
        .general_autofill_enabled(false)
        .initialization_script_for_all_frames(init_script());
    // Its network through the dead end alone, or no page. Windows: a browser of the pages' own,
    // and its filter as well, put once it is made. macOS: none (WKWebView's proxy needs macOS 14
    // and a feature of Tauri): the CSP, `on_navigation` and the guard hold it there.
    #[cfg(target_os = "linux")]
    {
        // Not `incognito`: wry gives an incognito WebKitGTK a context of its own, where Tauri,
        // which registers a scheme once per shared context, never registers this one (nor its
        // IPC). A context of its own instead, by a data folder new at each opening and gone at its
        // close: nothing lasts from one opening to the next either.
        let _ = data_root;
        let store = std::env::temp_dir().join(format!(
            "escouade-agentpage-{page}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        shelter
            .stores
            .lock()
            .insert(page.to_string(), store.clone());
        b = b.incognito(false).data_directory(store);
        let port = dead_end().ok_or("no dead end for the page's network")?;
        let proxy = Url::parse(&format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?;
        b = b.proxy_url(proxy);
    }
    #[cfg(windows)]
    {
        // A harness that forces WebView2's folder (`WEBVIEW2_USER_DATA_FOLDER`: the end-to-end
        // tests) leaves no room for a browser of other switches: the filter alone then.
        if std::env::var_os("WEBVIEW2_USER_DATA_FOLDER").is_none() {
            let port = dead_end().ok_or("no dead end for the page's network")?;
            b = b
                .data_directory(data_root.join("agent-pages-webview"))
                .additional_browser_args(&browser_args(port));
        } else {
            log::warn!("agent page {page}: WEBVIEW2_USER_DATA_FOLDER is set, no dead end (its filter alone)");
        }
    }
    #[cfg(target_os = "macos")]
    let _ = data_root;
    {
        let (shelter, tell, page, own) =
            (shelter.clone(), tell.clone(), page.to_string(), own.clone());
        b = b.on_navigation(move |to| {
            let verdict = judge(to, &page, windows, |u| own(u));
            if verdict != Verdict::Here {
                refused(&shelter, &page, to, verdict, &tell);
            }
            verdict == Verdict::Here
        });
    }
    {
        let (shelter, tell, page, own) =
            (shelter.clone(), tell.clone(), page.to_string(), own.clone());
        b = b.on_new_window(move |to, _| {
            let verdict = match judge(&to, &page, windows, |u| own(u)) {
                Verdict::Here => Verdict::Nowhere,
                v => v,
            };
            refused(&shelter, &page, &to, verdict, &tell);
            NewWindowResponse::Deny
        });
    }
    {
        let (shelter, tell, page) = (shelter.clone(), tell.clone(), page.to_string());
        b = b.on_download(move |_, _| {
            shelter.with_view(&page, |v| v.downloads_refused += 1);
            log::info!("agent page {page}: a download refused");
            tell(PageEvent {
                page: page.clone(),
                phase: "download",
                url: String::new(),
                title: String::new(),
            });
            false
        });
    }
    {
        let (shelter, tell, page) = (shelter.clone(), tell.clone(), page.to_string());
        b = b.on_permission_request(move |_, kind| {
            shelter.with_view(&page, |v| v.permissions_refused += 1);
            log::info!("agent page {page}: a permission refused ({kind:?})");
            tell(PageEvent {
                page: page.clone(),
                phase: "permission",
                url: String::new(),
                title: String::new(),
            });
            PermissionResponse::Deny
        });
    }
    {
        let (shelter, tell, page) = (shelter.clone(), tell.clone(), page.to_string());
        b = b.on_page_load(move |_, load| {
            let url = load.url().to_string();
            let finished = matches!(load.event(), PageLoadEvent::Finished);
            shelter.with_view(&page, |v| {
                v.url = url.clone();
                v.loaded = finished;
            });
            tell(PageEvent {
                page: page.clone(),
                phase: if finished { "finished" } else { "started" },
                url,
                title: String::new(),
            });
        });
    }
    {
        let (shelter, tell, page) = (shelter.clone(), tell.clone(), page.to_string());
        b = b.on_document_title_changed(move |_, title| {
            let title: String = title.chars().take(16_000).collect();
            shelter.with_view(&page, |v| v.title = title.clone());
            tell(PageEvent {
                page: page.clone(),
                phase: "title",
                url: String::new(),
                title,
            });
        });
    }
    Ok(b)
}

/// A refused address: a web one kept for the window to offer, anything else opened nowhere.
fn refused(shelter: &Shelter, page: &str, to: &Url, verdict: Verdict, tell: &Tell) {
    let outside = verdict == Verdict::Outside;
    shelter.with_view(page, |v| {
        v.navigations_refused += 1;
        if outside {
            v.wants = Some(to.to_string());
        }
    });
    log::info!(
        "agent page {page}: {}://{} refused",
        to.scheme(),
        to.host_str().unwrap_or("")
    );
    tell(PageEvent {
        page: page.to_string(),
        phase: if outside { "wants" } else { "refused" },
        url: if outside {
            to.to_string()
        } else {
            String::new()
        },
        title: String::new(),
    });
}

/// The web address page `page` asked to open, taken (the window's « Ouvrir dans le navigateur du
/// système » or « Ignorer »): the window opens the address the core kept, never one it is given.
pub fn take_wanted(shelter: &Shelter, page: &str) -> Option<String> {
    shelter
        .views
        .lock()
        .get_mut(page)
        .and_then(|v| v.wants.take())
}

/// Page `page` in the window: placed at `bounds` and shown, or hidden.
pub fn place<R: Runtime>(
    app: &AppHandle<R>,
    page: &str,
    bounds: Option<Bounds>,
) -> Result<(), String> {
    let Some(webview) = app.get_webview(&label(page)) else {
        return Ok(());
    };
    match bounds {
        Some(b) if b.width >= 1.0 && b.height >= 1.0 => {
            crate::devpage::set_bounds(&webview, b)?;
            webview.show().map_err(|e| e.to_string())
        }
        _ => webview.hide().map_err(|e| e.to_string()),
    }
}

/// Closes page `page`: its webview gone (and with it all it kept), the page still served.
pub fn close<R: Runtime>(app: &AppHandle<R>, shelter: &Shelter, page: &str) {
    if let Some(w) = app.get_webview(&label(page)) {
        let _ = w.close();
    }
    shelter.views.lock().remove(page);
    // Its webview's data folder (Linux), once WebKit has let it go.
    if let Some(store) = shelter.stores.lock().remove(page) {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let _ = std::fs::remove_dir_all(store);
        });
    }
}

/// The handler of the scheme, for the app's builder: a page served to its own webview alone.
pub fn register<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.register_asynchronous_uri_scheme_protocol(SCHEME, |ctx, request, responder| {
        let label = ctx.webview_label().to_string();
        let shelter = ctx
            .app_handle()
            .try_state::<Arc<Shelter>>()
            .map(|s| s.inner().clone());
        let method = request.method().as_str().to_string();
        let uri = request.uri().to_string();
        std::thread::spawn(move || {
            // Windows: a request that comes before the page's filter is on waits for it (5 s at
            // most) rather than being refused, which left WebView2 now and then on its error page
            // over the navigation that follows the filter (ESC-674). Nothing is served before it.
            if let (Some(s), Some(page)) = (shelter.as_ref(), page_of(&label)) {
                let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while s.unfiltered.lock().contains(page) && std::time::Instant::now() < until {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
            let answer = match shelter {
                Some(s) => s.serve(&label, &method, &uri, cfg!(windows)),
                None => answer(StatusCode::NOT_FOUND, "text/plain", Vec::new(), None),
            };
            responder.respond(answer);
        });
    })
}

/// The trap (`TRAP`), its listeners' ports put in: `tcp` for what reaches it by TCP, `udp` for
/// WebRTC's STUN.
pub fn trap(tcp: u16, udp: u16) -> String {
    TRAP.replace("__PORT__", &tcp.to_string())
        .replace("__UDP__", &udp.to_string())
}

/// Writes the trap at `dir` as a page's folder: `index.html` and `other.txt`, its own file.
pub fn write_trap(dir: &Path, tcp: u16, udp: u16) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("index.html"), trap(tcp, udp))?;
    std::fs::write(dir.join("other.txt"), "own file")
}

/// A neighbour's page, for the trap to try to read: `index.html` saying `secret of p-13`.
pub fn write_neighbour(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("index.html"),
        "<!doctype html><title>p-13</title><p>secret of p-13</p>",
    )
}
