//! The native read bridge of the browsers (ESC-668, AB-1; docs/AGENT-BROWSERS.md § 3 bis, 5.2, 6, 7):
//! what the core reads of a browser of « Navigateurs », for the MCP tools that come after it
//! (`list_browsers`, `read_browser`, `capture_browser`, `browser_console`, AB-2 to AB-4).
//!
//! The page is the developed app or a third party's site: Escouade does not trust it. So:
//! - **a world of Escouade's own**: the fixed scripts of `browserread.js` run in the isolated world
//!   `escouade-agent` of the main frame, by the webview's native handle (`Webview::with_webview`),
//!   never by `webview.eval` (the page's world, which lies: ESC-663's probe read `"menteur"` there).
//!   WebView2: `Page.createIsolatedWorld` then `Runtime.evaluate`; WebKitGTK:
//!   `run_javascript_in_world`; WKWebView: `evaluateJavaScript:inFrame:inContentWorld:`. The result
//!   comes back by the native callback. No script of an agent is ever run: a script is one of
//!   `Script`, its arguments encoded in JSON;
//! - **a closed list of the DevTools protocol's methods** (`Method`), under WebView2: no
//!   pass-through, for the probe read the cookies by `Network.getCookies` and the storage's token
//!   by `Runtime.evaluate` in the page's world. `Runtime.evaluate` goes to Escouade's world alone,
//!   with Escouade's library alone (`guard`);
//! - **captures** by each system (the view, an element, the whole document under WebView2 and
//!   WebKitGTK), 1 600 pixels at most on their longest side;
//! - **the console and the network's failures** kept from the creation of each browser's webview
//!   (`attach`), whether an agent reads them or not, in bounded buffers (`Buffers`: 500 entries of
//!   each), their addresses without their query (`strip_url`). WebView2 gives them by the protocol
//!   (`from: "browser"`: the page can neither silence nor forge them); WebKit has no such API, so a
//!   relay in the page (`browserread-relay.js`) keeps them, and the core asks for them from the
//!   isolated world every second, the relay answering inside that same call (`from: "page"`: the
//!   page can silence or forge it). Nothing waits in the isolated world between two calls: under
//!   WKWebView, it does not outlive an evaluation (ESC-692);
//! - **the HTML masked** (`mask_html`): the values of password and hidden fields, the `content` of
//!   a `<meta>` named for a token, the text of inline scripts, and the tokens that show (`mask_tokens`).
//!   Cookies, storage and headers are never read.
//!
//! Every call here waits for the webview's thread: never call them on it (the main thread).

use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Runtime, Webview};

/// The name of Escouade's isolated world, the same on every system.
pub const WORLD: &str = "escouade-agent";
/// What stands for a value that never leaves.
pub const MASKED: &str = "«masqué»";
/// The entries of a buffer (console, network), per browser.
pub const CAP: usize = 500;
/// The characters of an entry's text.
pub const TEXT_MAX: usize = 500;
/// The characters of a read, by default and at most.
pub const READ_DEFAULT: usize = 50_000;
pub const READ_MAX: usize = 200_000;
/// The longest side of a capture, in pixels; the height of a whole document, in CSS pixels.
pub const SHOT_SIDE: f64 = 1600.0;
pub const SHOT_DOC_HEIGHT: f64 = 8000.0;
/// The quality of a capture in JPEG (WebView2).
pub const JPEG_QUALITY: u32 = 70;
/// How long a call waits for the webview.
const WAIT: Duration = Duration::from_secs(10);
/// How often the core takes what WebKit's relay told.
#[cfg_attr(windows, allow(dead_code))]
const POLL: Duration = Duration::from_secs(1);

/// Escouade's library of fixed scripts, run in its world.
const LIB: &str = include_str!("browserread.js");
/// The console relay of a page under WebKit, run in the page's world at the start of each
/// document (an initialization script of the browser's webview, `devpage`).
pub const RELAY: &str = include_str!("browserread-relay.js");

// ---------- the closed list of the DevTools protocol (WebView2) ----------

/// The methods of the DevTools protocol the core may call, and none other (§ 5.2). AB-1 calls the
/// reading ones; the others are for the driving tickets (AB-7, AB-8), listed now so that the list
/// is one, closed, here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    PageCaptureScreenshot,
    PageCreateIsolatedWorld,
    PageGetFrameTree,
    PageNavigate,
    PageReload,
    RuntimeEvaluate,
    InputDispatchMouseEvent,
    InputDispatchKeyEvent,
    InputInsertText,
    OverlayHighlightNode,
    OverlayHighlightQuad,
    OverlayHideHighlight,
    OverlayEnable,
    DomEnable,
    EmulationSetDeviceMetricsOverride,
    EmulationClearDeviceMetricsOverride,
    AccessibilityGetFullAxTree,
    RuntimeEnable,
    LogEnable,
    NetworkEnable,
    /// The frames of another site, each in its own process, reached by a session of its own (ESC-682, AB-11):
    /// iframes alone, never paused, flattened.
    TargetSetAutoAttach,
}

impl Method {
    pub const ALL: [Method; 21] = [
        Method::PageCaptureScreenshot,
        Method::PageCreateIsolatedWorld,
        Method::PageGetFrameTree,
        Method::PageNavigate,
        Method::PageReload,
        Method::RuntimeEvaluate,
        Method::InputDispatchMouseEvent,
        Method::InputDispatchKeyEvent,
        Method::InputInsertText,
        Method::OverlayHighlightNode,
        Method::OverlayHighlightQuad,
        Method::OverlayHideHighlight,
        Method::OverlayEnable,
        Method::DomEnable,
        Method::EmulationSetDeviceMetricsOverride,
        Method::EmulationClearDeviceMetricsOverride,
        Method::AccessibilityGetFullAxTree,
        Method::RuntimeEnable,
        Method::LogEnable,
        Method::NetworkEnable,
        Method::TargetSetAutoAttach,
    ];

    /// Its name in the protocol.
    pub fn name(self) -> &'static str {
        match self {
            Method::PageCaptureScreenshot => "Page.captureScreenshot",
            Method::PageCreateIsolatedWorld => "Page.createIsolatedWorld",
            Method::PageGetFrameTree => "Page.getFrameTree",
            Method::PageNavigate => "Page.navigate",
            Method::PageReload => "Page.reload",
            Method::RuntimeEvaluate => "Runtime.evaluate",
            Method::InputDispatchMouseEvent => "Input.dispatchMouseEvent",
            Method::InputDispatchKeyEvent => "Input.dispatchKeyEvent",
            Method::InputInsertText => "Input.insertText",
            Method::OverlayHighlightNode => "Overlay.highlightNode",
            Method::OverlayHighlightQuad => "Overlay.highlightQuad",
            Method::OverlayHideHighlight => "Overlay.hideHighlight",
            Method::OverlayEnable => "Overlay.enable",
            Method::DomEnable => "DOM.enable",
            Method::EmulationSetDeviceMetricsOverride => "Emulation.setDeviceMetricsOverride",
            Method::EmulationClearDeviceMetricsOverride => "Emulation.clearDeviceMetricsOverride",
            Method::AccessibilityGetFullAxTree => "Accessibility.getFullAXTree",
            Method::RuntimeEnable => "Runtime.enable",
            Method::LogEnable => "Log.enable",
            Method::NetworkEnable => "Network.enable",
            Method::TargetSetAutoAttach => "Target.setAutoAttach",
        }
    }

    /// The method of the list that has this name: any other is refused.
    pub fn parse(name: &str) -> Result<Method, String> {
        Method::ALL
            .iter()
            .copied()
            .find(|m| m.name() == name)
            .ok_or_else(|| format!("not a method Escouade calls: {name}"))
    }
}

/// The events of the protocol the core listens to, from a browser's creation.
pub const EVENTS: [&str; 7] = [
    "Runtime.consoleAPICalled",
    "Runtime.exceptionThrown",
    "Log.entryAdded",
    "Network.requestWillBeSent",
    "Network.responseReceived",
    "Network.loadingFailed",
    "Network.loadingFinished",
];

/// Checks a call's parameters before it goes: `Runtime.evaluate` runs Escouade's library alone, in
/// Escouade's world alone (`world`, the context the core made for this browser), returning a value
/// and nothing of the page's console API; `Page.navigate` goes to a web address alone.
pub fn guard(method: Method, params: &Value, world: Option<i64>) -> Result<(), String> {
    let fields = params.as_object().ok_or("parameters: not an object")?;
    match method {
        Method::RuntimeEvaluate => {
            let ctx = fields.get("contextId").and_then(Value::as_i64);
            if ctx.is_none() || ctx != world {
                return Err("Runtime.evaluate: Escouade's world alone".into());
            }
            let ours = fields
                .get("expression")
                .and_then(Value::as_str)
                .is_some_and(|e| e.starts_with(&lib_head()) || e.starts_with(&drive_head()));
            if !ours {
                return Err("Runtime.evaluate: Escouade's scripts alone".into());
            }
            let allowed = ["expression", "contextId", "returnByValue", "silent"];
            if let Some(k) = fields.keys().find(|k| !allowed.contains(&k.as_str())) {
                return Err(format!("Runtime.evaluate: {k} is not allowed"));
            }
            Ok(())
        }
        Method::PageNavigate => {
            let web = fields
                .get("url")
                .and_then(Value::as_str)
                .and_then(|u| tauri::Url::parse(u).ok())
                .is_some_and(|u| matches!(u.scheme(), "http" | "https"));
            if web {
                Ok(())
            } else {
                Err("Page.navigate: a web address alone".into())
            }
        }
        // The driving's inputs (ESC-679): a click, a wheel, a key of the closed list, a text of 2 000 characters.
        Method::InputDispatchMouseEvent => {
            let t = fields.get("type").and_then(Value::as_str).unwrap_or("");
            if !matches!(
                t,
                "mouseMoved" | "mousePressed" | "mouseReleased" | "mouseWheel"
            ) {
                return Err(format!("Input.dispatchMouseEvent: {t} is not allowed"));
            }
            let at = ["x", "y"].iter().all(|k| {
                fields
                    .get(*k)
                    .and_then(Value::as_f64)
                    .is_some_and(|v| (0.0..=20_000.0).contains(&v))
            });
            if !at {
                return Err("Input.dispatchMouseEvent: a point of the view".into());
            }
            Ok(())
        }
        Method::InputDispatchKeyEvent => {
            let t = fields.get("type").and_then(Value::as_str).unwrap_or("");
            if !matches!(t, "keyDown" | "rawKeyDown" | "keyUp") {
                return Err(format!("Input.dispatchKeyEvent: {t} is not allowed"));
            }
            let key = fields.get("key").and_then(Value::as_str).unwrap_or("");
            if crate::browserdrive::Key::parse(key).is_none() {
                return Err(format!(
                    "Input.dispatchKeyEvent: {key} is not a key of the list"
                ));
            }
            if let Some(k) = fields.keys().find(|k| {
                ![
                    "type",
                    "key",
                    "code",
                    "windowsVirtualKeyCode",
                    "nativeVirtualKeyCode",
                    "text",
                    "unmodifiedText",
                ]
                .contains(&k.as_str())
            }) {
                return Err(format!("Input.dispatchKeyEvent: {k} is not allowed"));
            }
            Ok(())
        }
        Method::InputInsertText => {
            let ok = fields
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|t| t.chars().count() <= crate::browserdrive::TEXT_MAX)
                && fields.len() == 1;
            if ok {
                Ok(())
            } else {
                Err("Input.insertText: a text of 2 000 characters alone".into())
            }
        }
        Method::TargetSetAutoAttach => {
            if *params == auto_attach() {
                Ok(())
            } else {
                Err("Target.setAutoAttach: iframes alone, never paused".into())
            }
        }
        Method::EmulationSetDeviceMetricsOverride => {
            let w = fields.get("width").and_then(Value::as_i64).unwrap_or(0);
            if (200..=4000).contains(&w) {
                Ok(())
            } else {
                Err("Emulation.setDeviceMetricsOverride: a width between 200 and 4 000".into())
            }
        }
        // What the user sees of the driving (ESC-680, AB-8), painted by the browser over the page and absent from its
        // DOM: the highlight of the node Escouade's world found (its object alone), the ghost cursor (a quad of the
        // view), and the two domains they need, without parameters.
        Method::OverlayHighlightNode => {
            let object = fields
                .get("objectId")
                .and_then(Value::as_str)
                .is_some_and(|o| !o.is_empty() && o.len() <= 200);
            if !object {
                return Err("Overlay.highlightNode: a node of Escouade's world alone".into());
            }
            if let Some(k) = fields
                .keys()
                .find(|k| !["objectId", "highlightConfig"].contains(&k.as_str()))
            {
                return Err(format!("Overlay.highlightNode: {k} is not allowed"));
            }
            Ok(())
        }
        Method::OverlayHighlightQuad => {
            let quad = fields
                .get("quad")
                .and_then(Value::as_array)
                .is_some_and(|q| {
                    q.len() == 8
                        && q.iter()
                            .all(|v| v.as_f64().is_some_and(|v| (-100.0..=20_000.0).contains(&v)))
                });
            if !quad {
                return Err("Overlay.highlightQuad: four points of the view".into());
            }
            if let Some(k) = fields
                .keys()
                .find(|k| !["quad", "color", "outlineColor"].contains(&k.as_str()))
            {
                return Err(format!("Overlay.highlightQuad: {k} is not allowed"));
            }
            Ok(())
        }
        Method::OverlayEnable | Method::DomEnable | Method::OverlayHideHighlight => {
            if fields.is_empty() {
                Ok(())
            } else {
                Err(format!("{}: no parameter", method.name()))
            }
        }
        _ => Ok(()),
    }
}

/// The parameters of `Target.setAutoAttach`, the only ones (ESC-682): the frames of another site, each in its own
/// process (an artifact of claude.ai in its frame), attached flat, never waiting for a debugger.
pub fn auto_attach() -> Value {
    json!({
        "autoAttach": true,
        "waitForDebuggerOnStart": false,
        "flatten": true,
        "filter": [{ "type": "iframe", "exclude": false }],
    })
}

/// What may be called in the session of a frame (ESC-682): its frame tree, Escouade's world in it, and Escouade's
/// scripts in that world. Nothing else: no input, no capture, no console, no network of the frame.
#[cfg(any(windows, test))]
pub fn guard_frame(method: Method, params: &Value, world: Option<i64>) -> Result<(), String> {
    match method {
        Method::PageGetFrameTree | Method::PageCreateIsolatedWorld | Method::RuntimeEvaluate => {
            guard(method, params, world)
        }
        m => Err(format!("{}: not in a frame's session", m.name())),
    }
}

/// A child frame of a browser's document, as the protocol says it: its id, its address, and the session that
/// reaches it when it lives in its own process.
#[cfg(any(windows, test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub id: String,
    pub url: String,
    pub session: Option<String>,
}

/// The frame of the `<iframe>` the agent named: its `src` (resolved), and its rank among the document's iframes of
/// that origin. The same origin first, by rank; a single frame of the document when its address went elsewhere
/// (a redirect). None when it cannot be told.
#[cfg(any(windows, test))]
pub fn pick_frame<'a>(frames: &'a [Frame], src: &str, rank: usize) -> Option<&'a Frame> {
    let origin = |u: &str| {
        tauri::Url::parse(u)
            .ok()
            .map(|u| u.origin().ascii_serialization())
            .filter(|o| o != "null")
    };
    let want = origin(src);
    let same: Vec<&Frame> = frames
        .iter()
        .filter(|f| want.is_some() && origin(&f.url) == want)
        .collect();
    match same.len() {
        0 if frames.len() == 1 => frames.first(),
        0 => None,
        1 => same.first().copied(),
        _ => same.get(rank).copied(),
    }
}

// ---------- the fixed scripts ----------

/// What a read gives: the HTML (`outerHTML`) or the visible text (`innerText`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum What {
    #[default]
    Html,
    Text,
}

/// The fixed scripts of Escouade's library, the only ones the core runs in a page.
#[derive(Debug, Clone, PartialEq)]
pub enum Script<'a> {
    /// The HTML or the text of the elements a selector matches, or of the document.
    Read {
        selector: Option<&'a str>,
        what: What,
    },
    /// The view's size and scroll, and the box of the first visible element a selector matches.
    Rect { selector: Option<&'a str> },
    /// What WebKit's relay kept since the last time (`browserread-relay.js`), asked of it.
    Drain,
    /// How many calls Escouade's world has seen since it was made (the probe's measure, ESC-692).
    World,
}

/// The start of every expression the core runs: its library, called.
fn lib_head() -> String {
    format!("({LIB}")
}

/// The start of the driving's expressions (`browserdrive`, ESC-679): its own library, called.
fn drive_head() -> String {
    format!("({}", crate::browserdrive::LIB)
}

/// The expression of `script`: the library called with its arguments, values in JSON.
pub fn expression(script: &Script<'_>) -> String {
    let args = match script {
        Script::Read { selector, what } => {
            json!({ "run": "read", "selector": selector, "what": what })
        }
        Script::Rect { selector } => json!({ "run": "rect", "selector": selector }),
        Script::Drain => json!({ "run": "drain" }),
        Script::World => json!({ "run": "world" }),
    };
    format!("{})({args})", lib_head())
}

/// A selector an agent may give: 200 characters at most, without `:has()`, four levels at most.
pub fn check_selector(selector: &str) -> Result<(), String> {
    if selector.trim().is_empty() {
        return Err("an empty selector".into());
    }
    if selector.chars().count() > 200 {
        return Err("a selector of more than 200 characters".into());
    }
    if selector.to_ascii_lowercase().contains(":has(") {
        return Err("a selector with :has()".into());
    }
    // The compound selectors of each group, outside quotes, brackets and parentheses.
    let (mut depth, mut quote, mut levels, mut in_compound) = (0i32, None::<char>, 0usize, false);
    let mut worst = 0usize;
    for c in selector.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            _ => {}
        }
        if depth > 0 || matches!(c, '"' | '\'' | ']' | ')') {
            if !in_compound {
                in_compound = true;
                levels += 1;
            }
            continue;
        }
        if c == ',' {
            worst = worst.max(levels);
            levels = 0;
            in_compound = false;
        } else if c.is_whitespace() || matches!(c, '>' | '+' | '~') {
            in_compound = false;
        } else if !in_compound {
            in_compound = true;
            levels += 1;
        }
    }
    if quote.is_some() || depth != 0 {
        return Err("a selector not closed".into());
    }
    if worst.max(levels) > 4 {
        return Err("a selector of more than four levels".into());
    }
    Ok(())
}

// ---------- masking ----------

/// The HTML a browser serialized, with what never leaves masked: the `value` of a password or a
/// hidden field (often a token against forgery), the `content` of a `<meta>` named for a token
/// (`csrf`, `xsrf`, `token`, `nonce`), the text of inline `<script>` (their `src` stays), and the
/// tokens that show (`mask_tokens`). Each replaced by `«masqué»` and counted.
pub fn mask_html(html: &str) -> (String, usize) {
    let mut out = String::with_capacity(html.len());
    let mut masked = 0usize;
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(after) = rest.strip_prefix("<!--") {
            let end = after.find("-->").map(|e| e + 3).unwrap_or(after.len());
            out.push_str("<!--");
            out.push_str(&after[..end]);
            rest = &after[end..];
            continue;
        }
        let Some(tag) = parse_tag(rest) else {
            out.push('<');
            rest = &rest[1..];
            continue;
        };
        let (written, n) = tag.masked();
        out.push_str(&written);
        masked += n;
        rest = &rest[tag.len..];
        if tag.name == "script" && !tag.closing {
            let end = find_ci(rest, "</script").unwrap_or(rest.len());
            let body = &rest[..end];
            if !body.trim().is_empty() && body.trim() != MASKED {
                out.push_str(MASKED);
                masked += 1;
            } else {
                out.push_str(body);
            }
            rest = &rest[end..];
        }
    }
    out.push_str(rest);
    let (out, tokens) = mask_tokens(&out);
    (out, masked + tokens)
}

/// The first place of `needle` (ASCII) in `hay`, whatever its case.
fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    (0..h.len().saturating_sub(n.len() - 1)).find(|&i| h[i..i + n.len()].eq_ignore_ascii_case(n))
}

/// A start or end tag, as read at the start of a text.
struct Tag<'a> {
    text: &'a str,
    name: String,
    closing: bool,
    /// Its attributes: name (lower case) and the span of its value in `text`, quotes excluded.
    attrs: Vec<(String, Option<(usize, usize)>)>,
    len: usize,
}

fn parse_tag(s: &str) -> Option<Tag<'_>> {
    let b = s.as_bytes();
    let mut i = 1;
    let closing = b.get(i) == Some(&b'/');
    if closing {
        i += 1;
    }
    if !b.get(i)?.is_ascii_alphabetic() {
        return None;
    }
    let start = i;
    while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' && b[i] != b'/' {
        i += 1;
    }
    let name = s[start..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    loop {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        if i >= b.len() {
            return None;
        }
        if b[i] == b'>' {
            return Some(Tag {
                text: &s[..=i],
                name,
                closing,
                attrs,
                len: i + 1,
            });
        }
        let a = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b'=' | b'>' | b'/') {
            i += 1;
        }
        let attr = s[a..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if b.get(i) != Some(&b'=') {
            attrs.push((attr, None));
            continue;
        }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        match b.get(i) {
            Some(&q) if q == b'"' || q == b'\'' => {
                let v = i + 1;
                let end = s[v..].find(q as char)? + v;
                attrs.push((attr, Some((v, end))));
                i = end + 1;
            }
            Some(_) => {
                let v = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                    i += 1;
                }
                attrs.push((attr, Some((v, i))));
            }
            None => return None,
        }
    }
}

impl Tag<'_> {
    fn value(&self, attr: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(a, _)| a == attr)
            .and_then(|(_, v)| v.map(|(s, e)| &self.text[s..e]))
    }

    /// The tag written again, the values that never leave masked; and how many.
    fn masked(&self) -> (String, usize) {
        let hide: &[&str] = match self.name.as_str() {
            "input"
                if self.value("type").is_some_and(|t| {
                    t.eq_ignore_ascii_case("password") || t.eq_ignore_ascii_case("hidden")
                }) =>
            {
                &["value"]
            }
            "meta"
                if ["name", "property", "http-equiv"].iter().any(|a| {
                    self.value(a).is_some_and(|n| {
                        let n = n.to_ascii_lowercase();
                        ["csrf", "xsrf", "token", "nonce"]
                            .iter()
                            .any(|w| n.contains(w))
                    })
                }) =>
            {
                &["content"]
            }
            _ => &[],
        };
        let mut out = String::with_capacity(self.text.len());
        let (mut at, mut n) = (0usize, 0usize);
        for (attr, span) in &self.attrs {
            let Some((s, e)) = *span else { continue };
            if !hide.contains(&attr.as_str()) || &self.text[s..e] == MASKED {
                continue;
            }
            let quoted = s > 0 && matches!(self.text.as_bytes()[s - 1], b'"' | b'\'');
            out.push_str(&self.text[at..s]);
            if quoted {
                out.push_str(MASKED);
            } else {
                out.push('"');
                out.push_str(MASKED);
                out.push('"');
            }
            at = e;
            n += 1;
        }
        out.push_str(&self.text[at..]);
        (out, n)
    }
}

/// A text with the tokens that show masked: a key or a token of the usual forms (GitHub, GitLab,
/// OpenAI and Anthropic, Slack, AWS, a JSON Web Token, a `Bearer`'s, a private key's block). Each
/// replaced by `«masqué»` and counted.
pub fn mask_tokens(text: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut masked = 0usize;
    let mut rest = text;
    // A private key's block, whole.
    while let Some(at) = rest.find("-----BEGIN") {
        let block = &rest[at..];
        let end = block
            .find("-----END")
            .and_then(|e| block[e + 8..].find("-----").map(|f| e + 8 + f + 5));
        match end {
            Some(end) if block[..end].contains("PRIVATE KEY") => {
                out.push_str(&rest[..at]);
                out.push_str(MASKED);
                masked += 1;
                rest = &block[end..];
            }
            _ => {
                out.push_str(&rest[..at + 10]);
                rest = &rest[at + 10..];
            }
        }
    }
    out.push_str(rest);
    let text = out;
    let mut out = String::with_capacity(text.len());
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    let mut chars = text.char_indices().peekable();
    let mut last = 0usize;
    let mut bearer = false;
    while let Some((i, c)) = chars.next() {
        if !word(c) {
            continue;
        }
        let mut end = i + c.len_utf8();
        while let Some(&(j, d)) = chars.peek() {
            // A JSON Web Token's dots stay in its word.
            if word(d) || (d == '.' && text[i..].starts_with("eyJ")) {
                end = j + d.len_utf8();
                chars.next();
            } else {
                break;
            }
        }
        let w = text[i..end].trim_end_matches('.');
        let end = i + w.len();
        let secret = secret_word(w) || (bearer && w.len() >= 20);
        bearer = w.eq_ignore_ascii_case("bearer");
        if secret {
            out.push_str(&text[last..i]);
            out.push_str(MASKED);
            last = end;
            masked += 1;
        }
    }
    out.push_str(&text[last..]);
    (out, masked)
}

/// A word (letters, digits, `_`, `-`, the dots of a JSON Web Token) that is a key or a token.
fn secret_word(w: &str) -> bool {
    let parts: Vec<&str> = w.split('.').collect();
    if parts.len() == 3 && w.starts_with("eyJ") && parts.iter().all(|p| p.len() >= 8) {
        return true;
    }
    !w.contains('.') && (crate::asks::looks_secret(w) || w.starts_with("sk-ant-") && w.len() >= 24)
}

/// `text` cut to `max` characters, at a tag's border for HTML; whether it was cut, and its length.
pub fn clip(text: &str, max: usize, what: What) -> (String, bool, usize) {
    let total = text.chars().count();
    if total <= max {
        return (text.to_string(), false, total);
    }
    let cut = text
        .char_indices()
        .nth(max)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    let mut kept = &text[..cut];
    if what == What::Html {
        // After the last tag it holds whole: at a tag's border, nor a tag nor a text cut.
        if let Some(close) = kept.rfind('>') {
            kept = &kept[..=close];
        } else if let Some(amp) = kept.rfind('&') {
            // No tag at all: at least no entity cut.
            if kept.rfind(';').is_none_or(|semi| semi < amp) && kept.len() - amp <= 12 {
                kept = &kept[..amp];
            }
        }
    }
    (kept.to_string(), true, total)
}

/// An address as a buffer keeps it: without its query, its fragment or its user and password
/// (a token hides there often); a scheme other than the web's, by its name alone (`data:`).
pub fn strip_url(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    let kept = match tauri::Url::parse(text) {
        Ok(mut u) if matches!(u.scheme(), "http" | "https" | "ws" | "wss" | "file") => {
            u.set_query(None);
            u.set_fragment(None);
            let _ = u.set_username("");
            let _ = u.set_password(None);
            u.to_string()
        }
        Ok(u) => format!("{}:", u.scheme()),
        // A relative address: cut at its query or fragment.
        Err(_) => text.split(['?', '#']).next().unwrap_or("").to_string(),
    };
    clip_chars(&kept, TEXT_MAX)
}

fn clip_chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

// ---------- the buffers of the console and the network ----------

/// The level of a console's entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warn,
    Info,
    Log,
    Debug,
}

impl Level {
    fn of(name: &str) -> Level {
        match name {
            "error" | "assert" => Level::Error,
            "warning" | "warn" => Level::Warn,
            "info" => Level::Info,
            "debug" | "verbose" => Level::Debug,
            _ => Level::Log,
        }
    }
}

/// Who told an entry: the browser (the protocol of WebView2: the page can neither silence nor forge
/// it), or the page (WebKit's relay: it can).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Teller {
    Browser,
    Page,
}

/// An entry of the console.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConsoleEntry {
    #[serde(skip)]
    pub seq: u64,
    /// When, in milliseconds since 1970.
    pub t: u64,
    pub level: Level,
    /// 500 characters at most.
    pub text: String,
    /// `console`, `exception`, or the browser's (`network`, `security`, …).
    pub source: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub from: Teller,
}

/// A request that failed: a status of 400 or more, or no answer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NetEntry {
    #[serde(skip)]
    pub seq: u64,
    pub t: u64,
    pub method: String,
    /// Without its query (`strip_url`).
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// What was asked: `document`, `script`, `fetch`, `xhr`, …
    #[serde(rename = "type")]
    pub kind: String,
    pub from: Teller,
}

trait Stamped {
    fn stamp(&mut self, seq: u64, t: u64);
    fn seq(&self) -> u64;
    fn t(&self) -> u64;
}

impl Stamped for ConsoleEntry {
    fn stamp(&mut self, seq: u64, t: u64) {
        (self.seq, self.t) = (seq, t);
    }
    fn seq(&self) -> u64 {
        self.seq
    }
    fn t(&self) -> u64 {
        self.t
    }
}

impl Stamped for NetEntry {
    fn stamp(&mut self, seq: u64, t: u64) {
        (self.seq, self.t) = (seq, t);
    }
    fn seq(&self) -> u64 {
        self.seq
    }
    fn t(&self) -> u64 {
        self.t
    }
}

/// A buffer of `CAP` entries: the oldest go first. It keeps how many went, and when the last
/// `CAP` of them were told, to say what a reader lost.
struct Ring<T> {
    /// Each entry with the origin of the document the browser was on when it came (ESC-695).
    kept: VecDeque<(T, Arc<str>)>,
    /// The last number given (the first entry is 1).
    last: u64,
    last_t: u64,
    evicted: u64,
    evicted_t: VecDeque<u64>,
}

impl<T> Default for Ring<T> {
    fn default() -> Self {
        Ring {
            kept: VecDeque::new(),
            last: 0,
            last_t: 0,
            evicted: 0,
            evicted_t: VecDeque::new(),
        }
    }
}

impl<T: Stamped + Clone> Ring<T> {
    fn push(&mut self, mut entry: T, now: u64, origin: Arc<str>) {
        self.last += 1;
        // The times only go forward, for `since` to cut right.
        self.last_t = self.last_t.max(now);
        entry.stamp(self.last, self.last_t);
        if self.kept.len() >= CAP {
            if let Some((old, _)) = self.kept.pop_front() {
                self.evicted += 1;
                if self.evicted_t.len() >= CAP {
                    self.evicted_t.pop_front();
                }
                self.evicted_t.push_back(old.t());
            }
        }
        self.kept.push_back((entry, origin));
    }

    /// The entries after `since` that `keep` takes, of an origin `of` takes, `limit` at most; the
    /// number of the last one looked at; and how many after `since` were lost.
    fn read(
        &self,
        since: SinceAt,
        limit: usize,
        keep: impl Fn(&T) -> bool,
        of: &dyn Fn(&str) -> bool,
    ) -> (Vec<T>, u64, u64) {
        let after = |e: &T| match since {
            SinceAt::Seq(s) => e.seq() > s,
            SinceAt::Time(t) => e.t() >= t,
        };
        let dropped = match since {
            SinceAt::Seq(s) => self.evicted.saturating_sub(s),
            SinceAt::Time(t) => self.evicted_t.iter().filter(|&&et| et >= t).count() as u64,
        };
        let mut out = Vec::new();
        let mut cursor = match since {
            SinceAt::Seq(s) => s.min(self.last),
            SinceAt::Time(_) => self
                .kept
                .iter()
                .map(|(e, _)| e)
                .take_while(|e| !after(e))
                .last()
                .map_or(self.evicted, |e| e.seq()),
        };
        for (e, origin) in self.kept.iter().filter(|(e, _)| after(e)) {
            if out.len() >= limit {
                break;
            }
            cursor = e.seq();
            if keep(e) && of(origin) {
                out.push(e.clone());
            }
        }
        (out, cursor, dropped)
    }
}

/// Where a reader is in a browser's buffers: the last entry of each it took.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub console: u64,
    pub network: u64,
}

impl std::fmt::Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.console, self.network)
    }
}

impl std::str::FromStr for Cursor {
    type Err = String;
    fn from_str(s: &str) -> Result<Cursor, String> {
        let (c, n) = s.split_once('.').ok_or("not a cursor")?;
        Ok(Cursor {
            console: c.parse().map_err(|_| "not a cursor")?,
            network: n.parse().map_err(|_| "not a cursor")?,
        })
    }
}

/// From where a reader reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Since {
    /// Everything kept.
    #[default]
    Start,
    /// After a cursor a read gave.
    Cursor(Cursor),
    /// From a time, in milliseconds since 1970.
    Time(u64),
}

#[derive(Clone, Copy)]
enum SinceAt {
    Seq(u64),
    Time(u64),
}

/// What a reader asks of a browser's buffers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Query {
    pub since: Since,
    /// The levels it wants; all when empty.
    pub levels: Vec<Level>,
    /// The network's failures too.
    pub network: bool,
    /// The entries of each, 500 at most (200 when left at 0).
    pub limit: usize,
}

/// What a reader gets.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Logs {
    pub entries: Vec<ConsoleEntry>,
    pub network: Vec<NetEntry>,
    /// What to give as `since` to read what comes next.
    pub cursor: String,
    /// How many entries after `since` the buffers lost (they keep 500 of each).
    pub dropped: u64,
}

/// The request a failure will name (WebView2 tells its method and address before).
#[derive(Debug, Clone)]
struct Pending {
    method: String,
    url: String,
}

/// The console and the network's failures of a browser, kept from its webview's creation.
#[derive(Default)]
pub struct Buffers {
    inner: Mutex<Inner>,
    closed: AtomicBool,
}

#[derive(Default)]
struct Inner {
    /// The origin of the document the browser is on (`set_origin`), given to what comes now.
    origin: Arc<str>,
    console: Ring<ConsoleEntry>,
    network: Ring<NetEntry>,
    pending: HashMap<String, Pending>,
    pending_order: VecDeque<String>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The requests a buffer waits to know the end of, at most.
const PENDING_MAX: usize = 1000;

impl Buffers {
    /// Its browser is gone: the buffers stop taking anything.
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    pub fn closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn push_console(&self, mut e: ConsoleEntry) {
        if self.closed() {
            return;
        }
        e.text = clip_chars(&e.text, TEXT_MAX);
        e.url = strip_url(&e.url);
        e.source = clip_chars(&e.source, 40);
        let mut inner = self.inner.lock();
        let origin = inner.origin.clone();
        inner.console.push(e, now_ms(), origin);
    }

    pub fn push_network(&self, mut e: NetEntry) {
        if self.closed() {
            return;
        }
        e.url = strip_url(&e.url);
        e.method = clip_chars(&e.method, 16);
        e.kind = clip_chars(&e.kind, 40);
        e.error = e.error.map(|x| clip_chars(&x, 200));
        let mut inner = self.inner.lock();
        let origin = inner.origin.clone();
        inner.network.push(e, now_ms(), origin);
    }

    /// The browser goes to `url` (ESC-695): what comes from now on is of its origin
    /// (`scheme://host[:port]`, empty for another scheme).
    pub fn set_origin(&self, url: &str) {
        let origin = tauri::Url::parse(url)
            .ok()
            .filter(|u| matches!(u.scheme(), "http" | "https"))
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default();
        self.inner.lock().origin = Arc::from(origin);
    }

    /// What `q` asks, oldest first, of every origin.
    pub fn read(&self, q: &Query) -> Logs {
        self.read_of(q, &|_| true)
    }

    /// What `q` asks, oldest first, of the origins `of` takes alone (ESC-695): a tab goes to any
    /// web origin, and an agent reads only what came while it was on one it may read.
    pub fn read_of(&self, q: &Query, of: &dyn Fn(&str) -> bool) -> Logs {
        let limit = if q.limit == 0 { 200 } else { q.limit.min(CAP) };
        let inner = self.inner.lock();
        let (sc, sn) = match q.since {
            Since::Start => (SinceAt::Seq(0), SinceAt::Seq(0)),
            // A cursor beyond these buffers was given by the buffers of a webview made before them
            // (the browser opened again): everything they hold is after it.
            Since::Cursor(c) => (
                SinceAt::Seq(if c.console > inner.console.last {
                    0
                } else {
                    c.console
                }),
                SinceAt::Seq(if c.network > inner.network.last {
                    0
                } else {
                    c.network
                }),
            ),
            Since::Time(t) => (SinceAt::Time(t), SinceAt::Time(t)),
        };
        let (entries, console, dropped_c) = inner.console.read(
            sc,
            limit,
            |e| q.levels.is_empty() || q.levels.contains(&e.level),
            of,
        );
        let (network, net, dropped_n) = if q.network {
            inner.network.read(sn, limit, |_| true, of)
        } else {
            (Vec::new(), inner.network.last, 0)
        };
        Logs {
            entries,
            network,
            cursor: Cursor {
                console,
                network: net,
            }
            .to_string(),
            dropped: dropped_c + dropped_n,
        }
    }

    /// Takes an event of the DevTools protocol (WebView2), by its name and its parameters.
    pub fn take_cdp(&self, event: &str, p: &Value) {
        let text = |v: &Value| v.as_str().unwrap_or("").to_string();
        let line = |v: &Value| v.as_u64().map(|l| l as u32 + 1);
        match event {
            "Runtime.consoleAPICalled" => {
                let parts: Vec<String> = p["args"]
                    .as_array()
                    .map(|a| a.iter().map(remote_text).collect())
                    .unwrap_or_default();
                let frame = &p["stackTrace"]["callFrames"][0];
                self.push_console(ConsoleEntry {
                    seq: 0,
                    t: 0,
                    level: Level::of(p["type"].as_str().unwrap_or("log")),
                    text: parts.join(" "),
                    source: "console".into(),
                    url: text(&frame["url"]),
                    line: line(&frame["lineNumber"]),
                    from: Teller::Browser,
                });
            }
            "Runtime.exceptionThrown" => {
                let d = &p["exceptionDetails"];
                let said = d["exception"]["description"]
                    .as_str()
                    .or(d["text"].as_str())
                    .unwrap_or("");
                self.push_console(ConsoleEntry {
                    seq: 0,
                    t: 0,
                    level: Level::Error,
                    text: said.to_string(),
                    source: "exception".into(),
                    url: text(&d["url"]),
                    line: line(&d["lineNumber"]),
                    from: Teller::Browser,
                });
            }
            "Log.entryAdded" => {
                let e = &p["entry"];
                self.push_console(ConsoleEntry {
                    seq: 0,
                    t: 0,
                    level: Level::of(e["level"].as_str().unwrap_or("info")),
                    text: text(&e["text"]),
                    source: e["source"].as_str().unwrap_or("other").to_string(),
                    url: text(&e["url"]),
                    line: line(&e["lineNumber"]),
                    from: Teller::Browser,
                });
            }
            "Network.requestWillBeSent" => {
                let id = text(&p["requestId"]);
                let mut inner = self.inner.lock();
                if !inner.pending.contains_key(&id) {
                    inner.pending_order.push_back(id.clone());
                }
                inner.pending.insert(
                    id,
                    Pending {
                        method: text(&p["request"]["method"]),
                        url: strip_url(p["request"]["url"].as_str().unwrap_or("")),
                    },
                );
                while inner.pending_order.len() > PENDING_MAX {
                    if let Some(old) = inner.pending_order.pop_front() {
                        inner.pending.remove(&old);
                    }
                }
            }
            "Network.responseReceived" => {
                let status = p["response"]["status"].as_u64().unwrap_or(0);
                if status < 400 {
                    return;
                }
                let id = text(&p["requestId"]);
                let method = self.inner.lock().pending.get(&id).map(|r| r.method.clone());
                self.push_network(NetEntry {
                    seq: 0,
                    t: 0,
                    method: method.unwrap_or_else(|| "GET".into()),
                    url: text(&p["response"]["url"]),
                    status: Some(status.min(999) as u16),
                    error: None,
                    kind: kind_of(&p["type"]),
                    from: Teller::Browser,
                });
            }
            "Network.loadingFailed" => {
                let id = text(&p["requestId"]);
                let asked = {
                    let mut inner = self.inner.lock();
                    inner.pending_order.retain(|r| r != &id);
                    inner.pending.remove(&id)
                };
                let asked = asked.unwrap_or(Pending {
                    method: "GET".into(),
                    url: String::new(),
                });
                self.push_network(NetEntry {
                    seq: 0,
                    t: 0,
                    method: asked.method,
                    url: asked.url,
                    status: None,
                    error: Some(text(&p["errorText"])),
                    kind: kind_of(&p["type"]),
                    from: Teller::Browser,
                });
            }
            "Network.loadingFinished" => {
                let id = text(&p["requestId"]);
                let mut inner = self.inner.lock();
                if inner.pending.remove(&id).is_some() {
                    inner.pending_order.retain(|r| r != &id);
                }
            }
            _ => {}
        }
    }

    /// Takes what WebKit's relay told (the answer of `Script::Drain`): entries the page can forge.
    pub fn take_relay(&self, drained: &Value) {
        let Some(entries) = drained["entries"].as_array() else {
            return;
        };
        for raw in entries.iter().filter_map(Value::as_str) {
            let Ok(e) = serde_json::from_str::<Value>(raw) else {
                continue;
            };
            let text = |k: &str| e[k].as_str().unwrap_or("").to_string();
            match e["kind"].as_str() {
                Some("console") => self.push_console(ConsoleEntry {
                    seq: 0,
                    t: 0,
                    level: Level::of(e["level"].as_str().unwrap_or("log")),
                    text: text("text"),
                    source: if e["source"] == "exception" {
                        "exception".into()
                    } else {
                        "console".into()
                    },
                    url: text("url"),
                    line: e["line"]
                        .as_u64()
                        .filter(|l| *l > 0)
                        .map(|l| l.min(u32::MAX as u64) as u32),
                    from: Teller::Page,
                }),
                Some("network") => self.push_network(NetEntry {
                    seq: 0,
                    t: 0,
                    method: text("method").to_ascii_uppercase(),
                    url: text("url"),
                    status: e["status"].as_u64().map(|s| s.min(999) as u16),
                    error: e["error"].as_str().map(String::from),
                    kind: text("type"),
                    from: Teller::Page,
                }),
                _ => {}
            }
        }
    }
}

/// The text of an argument of `console.*`, as the protocol describes it.
fn remote_text(v: &Value) -> String {
    match &v["value"] {
        Value::String(s) => s.clone(),
        Value::Null => v["unserializableValue"]
            .as_str()
            .or(v["description"].as_str())
            .unwrap_or(v["type"].as_str().unwrap_or(""))
            .to_string(),
        other => other.to_string(),
    }
}

fn kind_of(v: &Value) -> String {
    v.as_str().unwrap_or("other").to_ascii_lowercase()
}

// ---------- reading a page ----------

/// What a read of a page gives (`read_browser`, AB-2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Read {
    pub url: String,
    pub title: String,
    pub selector: Option<String>,
    /// How many elements matched (1 for the document).
    pub matched: usize,
    pub what: What,
    /// The HTML or the text, masked, cut at `max_chars`.
    pub content: String,
    pub chars: usize,
    pub truncated: bool,
    pub total_chars: usize,
    /// How many values were masked.
    pub masked: usize,
}

/// Puts together what `Script::Read` answered: its parts end to end, masked, then cut.
pub fn assemble(
    answer: &Value,
    selector: Option<&str>,
    what: What,
    max_chars: usize,
) -> Result<Read, String> {
    if let Some(e) = answer["error"].as_str() {
        return Err(match e {
            "selector" => format!(
                "not a selector the page understands: {}",
                selector.unwrap_or("")
            ),
            _ => format!("the read failed: {e}"),
        });
    }
    let parts: Vec<&str> = answer["parts"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let joined = parts.join("\n");
    let (masked_text, masked) = match what {
        What::Html => mask_html(&joined),
        What::Text => mask_tokens(&joined),
    };
    let max = max_chars.clamp(1, READ_MAX);
    let (content, truncated, total_chars) = clip(&masked_text, max, what);
    Ok(Read {
        url: answer["url"].as_str().unwrap_or("").to_string(),
        title: crate::devpage::clip_title(answer["title"].as_str().unwrap_or("")),
        selector: selector.map(String::from),
        matched: answer["matched"].as_u64().unwrap_or(0) as usize,
        what,
        chars: content.chars().count(),
        content,
        truncated,
        total_chars,
        masked: masked + answer["masked"].as_u64().unwrap_or(0) as usize,
    })
}

/// Reads the HTML or the text of `selector` (the document without one) in the browser `webview`,
/// in Escouade's world, masked and cut at `max_chars` (50 000 by default, 200 000 at most).
pub fn read<R: Runtime>(
    webview: &Webview<R>,
    selector: Option<&str>,
    what: What,
    max_chars: Option<usize>,
) -> Result<Read, String> {
    if let Some(s) = selector {
        check_selector(s)?;
    }
    let answer = eval(webview, &Script::Read { selector, what })?;
    assemble(&answer, selector, what, max_chars.unwrap_or(READ_DEFAULT))
}

/// Runs a fixed script in Escouade's world of the browser `webview`; its answer.
pub fn eval<R: Runtime>(webview: &Webview<R>, script: &Script<'_>) -> Result<Value, String> {
    let text = platform::eval(webview, &expression(script))?;
    serde_json::from_str(&text).map_err(|e| format!("not an answer of Escouade's script: {e}"))
}

/// Runs an expression of the driving's library (`browserdrive::expression`) in Escouade's world: its text. Under
/// Windows the guard takes it for Escouade's own (it starts with the library) or refuses it.
pub fn eval_expression<R: Runtime>(
    webview: &Webview<R>,
    expression: &str,
) -> Result<String, String> {
    if !expression.starts_with(&drive_head()) && !expression.starts_with(&lib_head()) {
        return Err("Escouade's scripts alone".into());
    }
    platform::eval(webview, expression)
}

/// Runs an expression of the driving's library in Escouade's world of a child frame (ESC-682, AB-11): the frame of
/// the `<iframe>` whose resolved `src` and rank among the document's iframes of that origin are given. Under Windows
/// alone; `Err("unreached")` when the frame is not among the document's.
#[cfg(windows)]
pub fn eval_frame_expression<R: Runtime>(
    webview: &Webview<R>,
    src: &str,
    rank: usize,
    expression: &str,
) -> Result<String, String> {
    if !expression.starts_with(&drive_head()) {
        return Err("Escouade's scripts alone".into());
    }
    platform::eval_frame(webview, src, rank, expression)
}

/// An input or a width of the DevTools protocol for the driving (ESC-679), of the closed list and checked by `guard`.
#[cfg(windows)]
pub fn input<R: Runtime>(
    webview: &Webview<R>,
    method: Method,
    params: Value,
) -> Result<Value, String> {
    match method {
        Method::InputDispatchMouseEvent
        | Method::InputDispatchKeyEvent
        | Method::InputInsertText
        | Method::EmulationSetDeviceMetricsOverride
        | Method::EmulationClearDeviceMetricsOverride => platform::cdp(webview, method, params),
        _ => Err(format!("{}: not an input", method.name())),
    }
}

/// What the user sees of the driving under Windows (ESC-680, AB-8): the browser's overlay, of the closed list and
/// checked by `guard`. The page's script neither reads nor clears it: it is not in its DOM.
#[cfg(windows)]
pub fn overlay<R: Runtime>(
    webview: &Webview<R>,
    method: Method,
    params: Value,
) -> Result<Value, String> {
    match method {
        Method::DomEnable
        | Method::OverlayEnable
        | Method::OverlayHighlightNode
        | Method::OverlayHighlightQuad
        | Method::OverlayHideHighlight => platform::cdp(webview, method, params),
        _ => Err(format!("{}: not of the overlay", method.name())),
    }
}

/// The object of the element an expression of the driving's library finds in Escouade's world (`run: "node"`), for
/// the overlay to highlight; `None` when none is found.
#[cfg(windows)]
pub fn node<R: Runtime>(webview: &Webview<R>, expression: &str) -> Result<Option<String>, String> {
    if !expression.starts_with(&drive_head()) {
        return Err("Escouade's scripts alone".into());
    }
    platform::node(webview, expression)
}

// ---------- captures ----------

/// The view of a page as `Script::Rect` tells it, in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub inner_width: f64,
    pub inner_height: f64,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub dpr: f64,
    pub doc_width: f64,
    pub doc_height: f64,
}

/// A box, in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// What to capture.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Ask<'a> {
    /// The first visible element it matches; the view without one.
    pub selector: Option<&'a str>,
    /// The whole document, 8 000 CSS pixels high at most (not with a selector).
    pub full_page: bool,
    /// Another width, in CSS pixels, emulated for the capture alone (WebView2).
    pub width: Option<u32>,
}

/// A part of the page to capture: in CSS pixels of the view (`viewport`) or of the document, and
/// the scale that keeps its longest side at 1 600 pixels at most.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    /// Relative to the view's top left.
    pub viewport: Rect,
    /// Relative to the document's top left (the protocol of WebView2 clips by it).
    pub document: Rect,
    pub scale: f64,
    pub full_page: bool,
}

/// What part of the page a capture takes: an element (what of it shows in the view), the whole
/// document (8 000 CSS pixels high at most), or the view.
pub fn plan(view: View, element: Option<Rect>, full_page: bool) -> Result<Plan, String> {
    let viewport = match (element, full_page) {
        (Some(r), _) => {
            let (x0, y0) = (r.x.max(0.0), r.y.max(0.0));
            let (x1, y1) = (
                (r.x + r.width).min(view.inner_width),
                (r.y + r.height).min(view.inner_height),
            );
            if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
                return Err("the element is not in the view".into());
            }
            Rect {
                x: x0,
                y: y0,
                width: x1 - x0,
                height: y1 - y0,
            }
        }
        (None, true) => Rect {
            x: -view.scroll_x,
            y: -view.scroll_y,
            width: view.doc_width.max(view.inner_width),
            height: view.doc_height.max(view.inner_height).min(SHOT_DOC_HEIGHT),
        },
        (None, false) => Rect {
            x: 0.0,
            y: 0.0,
            width: view.inner_width,
            height: view.inner_height,
        },
    };
    let document = Rect {
        x: viewport.x + view.scroll_x,
        y: viewport.y + view.scroll_y,
        ..viewport
    };
    let dpr = if view.dpr > 0.0 { view.dpr } else { 1.0 };
    let side = viewport.width.max(viewport.height) * dpr;
    let scale = if side > SHOT_SIDE {
        SHOT_SIDE / side
    } else {
        1.0
    };
    Ok(Plan {
        viewport,
        document,
        scale,
        full_page: element.is_none() && full_page,
    })
}

/// The format of a capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Jpeg,
    Png,
}

/// A capture: its bytes, and what it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    pub bytes: Vec<u8>,
    pub format: Format,
    /// The part of the page taken, in CSS pixels.
    pub width: f64,
    pub height: f64,
    /// The image's pixels per CSS pixel.
    pub scale: f64,
}

/// Captures the browser `webview`: an element, the whole document or the view (§ 3 bis).
pub fn capture<R: Runtime>(webview: &Webview<R>, ask: Ask<'_>) -> Result<Shot, String> {
    if let Some(s) = ask.selector {
        check_selector(s)?;
    }
    if ask.selector.is_some() && ask.full_page {
        return Err("an element or the whole document, not both".into());
    }
    if let Some(w) = ask.width {
        if !(200..=4000).contains(&w) {
            return Err("a width between 200 and 4 000".into());
        }
    }
    platform::capture(webview, ask)
}

/// The most bytes of a capture's image given to an agent (ESC-670).
pub const SHOT_BYTES: usize = 600_000;

/// A capture as the agent gets it: a JPEG within the bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct Fitted {
    pub bytes: Vec<u8>,
    /// Its pixels.
    pub width: u32,
    pub height: u32,
    /// What its side was multiplied by to fit (1 when it fitted as it was).
    pub factor: f64,
}

/// `shot` in JPEG of quality 70, 1 600 pixels at most on its longest side and `SHOT_BYTES` at
/// most: a lower quality, then smaller, until it fits. A JPEG that already fits is kept as it is.
pub fn fit(shot: &Shot) -> Result<Fitted, String> {
    fit_within(shot, SHOT_BYTES)
}

pub(crate) fn fit_within(shot: &Shot, max_bytes: usize) -> Result<Fitted, String> {
    let format = match shot.format {
        Format::Jpeg => image::ImageFormat::Jpeg,
        Format::Png => image::ImageFormat::Png,
    };
    let img = image::load_from_memory_with_format(&shot.bytes, format)
        .map_err(|e| format!("not an image: {e}"))?;
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err("an empty capture".into());
    }
    let side = w.max(h) as f64;
    if shot.format == Format::Jpeg && side <= SHOT_SIDE && shot.bytes.len() <= max_bytes {
        return Ok(Fitted {
            bytes: shot.bytes.clone(),
            width: w,
            height: h,
            factor: 1.0,
        });
    }
    let rgb = image::DynamicImage::ImageRgb8(img.to_rgb8());
    let encode = |im: &image::DynamicImage, quality: u8| -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        im.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut out, quality,
        ))
        .map_err(|e| e.to_string())?;
        Ok(out)
    };
    let mut factor = (SHOT_SIDE / side).min(1.0);
    for _ in 0..12 {
        let (nw, nh) = (
            ((w as f64 * factor).round() as u32).max(1),
            ((h as f64 * factor).round() as u32).max(1),
        );
        let im = if (nw, nh) == (w, h) {
            rgb.clone()
        } else {
            rgb.resize_exact(nw, nh, image::imageops::FilterType::Triangle)
        };
        for quality in [JPEG_QUALITY as u8, 55, 40] {
            let bytes = encode(&im, quality)?;
            if bytes.len() <= max_bytes {
                return Ok(Fitted {
                    bytes,
                    width: nw,
                    height: nh,
                    factor,
                });
            }
        }
        factor *= 0.75;
    }
    Err("the capture does not fit in its bounds".into())
}

/// The view, and the box of `selector`, as the page shows them now.
fn measure<R: Runtime>(
    webview: &Webview<R>,
    selector: Option<&str>,
) -> Result<(View, Option<Rect>), String> {
    let answer = eval(webview, &Script::Rect { selector })?;
    if answer["error"].is_string() {
        return Err(format!(
            "not a selector the page understands: {}",
            selector.unwrap_or("")
        ));
    }
    let view: View = serde_json::from_value(answer["view"].clone()).map_err(|e| e.to_string())?;
    let element: Option<Rect> = serde_json::from_value(answer["box"].clone()).unwrap_or(None);
    if selector.is_some() && element.is_none() {
        return Err(format!("no visible element for {}", selector.unwrap_or("")));
    }
    Ok((view, element))
}

// ---------- from the creation of each webview ----------

/// Keeps the console and the network's failures of the browser `webview` from now on, in
/// `buffers`: called by `devpage::open` right after it made the webview. WebView2: the protocol's
/// events, enabled now (its console comes back whole: `Runtime.enable` and `Log.enable` give what
/// was told before them). WebKit: the relay of each document (`RELAY`, put in by `devpage`), whose
/// collector in Escouade's world the core empties every second.
pub fn attach<R: Runtime>(webview: &Webview<R>, buffers: Arc<Buffers>) {
    platform::attach(webview, buffers);
}

/// Forgets what the core kept for the browser `label` (its world): it is closed.
pub fn forget(label: &str) {
    #[cfg(windows)]
    {
        platform::worlds().lock().remove(label);
        platform::forget_frames(label);
    }
    #[cfg(not(windows))]
    let _ = label;
}

/// Waits for what the webview's thread sends back.
fn wait<T>(rx: std::sync::mpsc::Receiver<Result<T, String>>, what: &str) -> Result<T, String> {
    rx.recv_timeout(WAIT)
        .map_err(|_| format!("{what}: no answer from the webview"))?
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::sync::OnceLock;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2, ICoreWebView2DevToolsProtocolEventReceivedEventArgs,
    };
    use webview2_com::{
        take_pwstr, CallDevToolsProtocolMethodCompletedHandler,
        DevToolsProtocolEventReceivedEventHandler,
    };
    use windows_core::{HSTRING, PWSTR};

    /// Escouade's world in each browser, by label: made once per document.
    pub fn worlds() -> &'static Mutex<HashMap<String, i64>> {
        static WORLDS: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
        WORLDS.get_or_init(Default::default)
    }

    /// The sessions of the frames of another site that live in their own process, by browser label: the session,
    /// the frame's id (its target's), as `Target.attachedToTarget` said them (ESC-682).
    type Sessions = Mutex<HashMap<String, Vec<(String, String)>>>;
    fn sessions() -> &'static Sessions {
        static SESSIONS: OnceLock<Sessions> = OnceLock::new();
        SESSIONS.get_or_init(Default::default)
    }

    /// Escouade's world in a child frame, by browser label and frame id.
    fn frame_worlds() -> &'static Mutex<HashMap<(String, String), i64>> {
        static WORLDS: OnceLock<Mutex<HashMap<(String, String), i64>>> = OnceLock::new();
        WORLDS.get_or_init(Default::default)
    }

    /// Whether an event comes from a frame's own session rather than the browser's (ESC-682).
    fn from_frame(a: &ICoreWebView2DevToolsProtocolEventReceivedEventArgs) -> bool {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2DevToolsProtocolEventReceivedEventArgs2;
        use windows_core::Interface;
        let Ok(a2) = a.cast::<ICoreWebView2DevToolsProtocolEventReceivedEventArgs2>() else {
            return false;
        };
        let mut s = PWSTR::null();
        // SAFETY: the event's arguments, alive for the call; the string is taken (freed) at once.
        if unsafe { a2.SessionId(&mut s) }.is_err() {
            return false;
        }
        !take_pwstr(s).is_empty()
    }

    /// What a browser closing leaves: its frames' sessions and worlds.
    pub fn forget_frames(label: &str) {
        sessions().lock().remove(label);
        frame_worlds().lock().retain(|(l, _), _| l != label);
    }

    /// Calls a method of the closed list on the webview, in the process (no debugging port).
    pub fn cdp<R: Runtime>(
        webview: &Webview<R>,
        method: Method,
        params: Value,
    ) -> Result<Value, String> {
        let world = worlds().lock().get(webview.label()).copied();
        guard(method, &params, world)?;
        call(webview, None, method, params)
    }

    /// Calls a method in the session of a child frame (`session`), or in the browser's own (`None`), for Escouade's
    /// world `world` of that frame: `guard_frame` first (ESC-682).
    fn cdp_frame<R: Runtime>(
        webview: &Webview<R>,
        session: Option<&str>,
        method: Method,
        params: Value,
        world: Option<i64>,
    ) -> Result<Value, String> {
        guard_frame(method, &params, world)?;
        call(webview, session, method, params)
    }

    fn call<R: Runtime>(
        webview: &Webview<R>,
        session: Option<&str>,
        method: Method,
        params: Value,
    ) -> Result<Value, String> {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_11;
        use windows_core::Interface;
        let (tx, rx) = std::sync::mpsc::channel();
        let name = method.name();
        let body = params.to_string();
        let session = session.map(String::from);
        webview
            .with_webview(move |platform| {
                let failed = tx.clone();
                let handler =
                    CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |r, s| {
                        let _ = tx.send(match r {
                            Ok(()) => serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()),
                            Err(e) => Err(format!("{name}: {e} {s}")),
                        });
                        Ok(())
                    }));
                // SAFETY: the controller is the live one of this WebView, used on its thread.
                let sent = unsafe {
                    platform
                        .controller()
                        .CoreWebView2()
                        .and_then(|core| match &session {
                            None => core.CallDevToolsProtocolMethod(
                                &HSTRING::from(name),
                                &HSTRING::from(body),
                                &handler,
                            ),
                            Some(s) => core.cast::<ICoreWebView2_11>().and_then(|core| {
                                core.CallDevToolsProtocolMethodForSession(
                                    &HSTRING::from(s.as_str()),
                                    &HSTRING::from(name),
                                    &HSTRING::from(body),
                                    &handler,
                                )
                            }),
                        })
                };
                if let Err(e) = sent {
                    let _ = failed.send(Err(format!("{name}: {e}")));
                }
            })
            .map_err(|e| e.to_string())?;
        wait(rx, name)
    }

    /// The child frames of the browser's document: those of its process (its frame tree), and those of another
    /// site in their own (a session each, its address read in it).
    fn frames<R: Runtime>(webview: &Webview<R>) -> Result<Vec<Frame>, String> {
        let tree = cdp(webview, Method::PageGetFrameTree, json!({}))?;
        let mut out: Vec<Frame> = tree["frameTree"]["childFrames"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|c| {
                Some(Frame {
                    id: c["frame"]["id"].as_str()?.to_string(),
                    url: c["frame"]["url"].as_str().unwrap_or("").to_string(),
                    session: None,
                })
            })
            .collect();
        let attached = sessions()
            .lock()
            .get(webview.label())
            .cloned()
            .unwrap_or_default();
        for (session, id) in attached {
            let Ok(t) = cdp_frame(
                webview,
                Some(&session),
                Method::PageGetFrameTree,
                json!({}),
                None,
            ) else {
                continue;
            };
            let url = t["frameTree"]["frame"]["url"].as_str().unwrap_or("");
            let id = t["frameTree"]["frame"]["id"]
                .as_str()
                .unwrap_or(&id)
                .to_string();
            match out.iter_mut().find(|f| f.id == id) {
                Some(f) => {
                    f.session = Some(session);
                    f.url = url.to_string();
                }
                None => out.push(Frame {
                    id,
                    url: url.to_string(),
                    session: Some(session),
                }),
            }
        }
        Ok(out)
    }

    /// Escouade's world in a child frame: the one made for its document, or a new one.
    fn frame_world<R: Runtime>(
        webview: &Webview<R>,
        frame: &Frame,
        fresh: bool,
    ) -> Result<i64, String> {
        let key = (webview.label().to_string(), frame.id.clone());
        if !fresh {
            if let Some(ctx) = frame_worlds().lock().get(&key) {
                return Ok(*ctx);
            }
        }
        let made = cdp_frame(
            webview,
            frame.session.as_deref(),
            Method::PageCreateIsolatedWorld,
            json!({ "frameId": frame.id, "worldName": WORLD, "grantUniveralAccess": false }),
            None,
        )?;
        let ctx = made["executionContextId"]
            .as_i64()
            .ok_or("no isolated world in the frame")?;
        frame_worlds().lock().insert(key, ctx);
        Ok(ctx)
    }

    /// Runs an expression of Escouade's scripts in its world of the child frame of the `<iframe>` whose `src` and
    /// rank are given (ESC-682): its text. `Err("unreached")` when the frame is not found among the document's.
    pub fn eval_frame<R: Runtime>(
        webview: &Webview<R>,
        src: &str,
        rank: usize,
        expression: &str,
    ) -> Result<String, String> {
        let all = frames(webview)?;
        let frame = pick_frame(&all, src, rank).ok_or("unreached")?.clone();
        log::info!(
            "a frame of {} reached {}",
            webview.label(),
            if frame.session.is_some() {
                "by its own session"
            } else {
                "in the page's process"
            }
        );
        let mut last = String::new();
        for fresh in [false, true] {
            let ctx = frame_world(webview, &frame, fresh)?;
            let answer = cdp_frame(
                webview,
                frame.session.as_deref(),
                Method::RuntimeEvaluate,
                json!({ "expression": expression, "contextId": ctx, "returnByValue": true, "silent": true }),
                Some(ctx),
            );
            match answer {
                Ok(v) if v.get("exceptionDetails").is_none() => {
                    return v["result"]["value"]
                        .as_str()
                        .map(String::from)
                        .ok_or_else(|| "no answer".into())
                }
                Ok(v) => {
                    last = v["exceptionDetails"]["text"]
                        .as_str()
                        .unwrap_or("threw")
                        .to_string()
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    /// Escouade's world in the main frame of the browser: the one made for its document, or a new one.
    fn world<R: Runtime>(webview: &Webview<R>, fresh: bool) -> Result<i64, String> {
        if !fresh {
            if let Some(ctx) = worlds().lock().get(webview.label()) {
                return Ok(*ctx);
            }
        }
        let tree = cdp(webview, Method::PageGetFrameTree, json!({}))?;
        let frame = tree["frameTree"]["frame"]["id"]
            .as_str()
            .ok_or("no main frame")?
            .to_string();
        let made = cdp(
            webview,
            Method::PageCreateIsolatedWorld,
            json!({ "frameId": frame, "worldName": WORLD, "grantUniveralAccess": false }),
        )?;
        let ctx = made["executionContextId"]
            .as_i64()
            .ok_or("no isolated world")?;
        worlds().lock().insert(webview.label().to_string(), ctx);
        Ok(ctx)
    }

    pub fn eval<R: Runtime>(webview: &Webview<R>, expression: &str) -> Result<String, String> {
        let mut last = String::new();
        // The world of a document is gone with it: made again once, after a navigation.
        for fresh in [false, true] {
            let ctx = world(webview, fresh)?;
            let answer = cdp(
                webview,
                Method::RuntimeEvaluate,
                json!({ "expression": expression, "contextId": ctx, "returnByValue": true, "silent": true }),
            );
            match answer {
                Ok(v) if v.get("exceptionDetails").is_none() => {
                    return v["result"]["value"]
                        .as_str()
                        .map(String::from)
                        .ok_or_else(|| "no answer".into())
                }
                Ok(v) => {
                    last = v["exceptionDetails"]["text"]
                        .as_str()
                        .unwrap_or("threw")
                        .to_string()
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    /// The remote object of the element the expression gives, evaluated in Escouade's world (not by value): for the
    /// overlay alone. `None` when it gives no element.
    pub fn node<R: Runtime>(
        webview: &Webview<R>,
        expression: &str,
    ) -> Result<Option<String>, String> {
        let mut last = String::new();
        for fresh in [false, true] {
            let ctx = world(webview, fresh)?;
            let answer = cdp(
                webview,
                Method::RuntimeEvaluate,
                json!({ "expression": expression, "contextId": ctx, "returnByValue": false, "silent": true }),
            );
            match answer {
                Ok(v) if v.get("exceptionDetails").is_none() => {
                    let r = &v["result"];
                    return Ok(if r["subtype"] == "node" {
                        r["objectId"].as_str().map(String::from)
                    } else {
                        None
                    });
                }
                Ok(v) => {
                    last = v["exceptionDetails"]["text"]
                        .as_str()
                        .unwrap_or("threw")
                        .to_string()
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    pub fn capture<R: Runtime>(webview: &Webview<R>, ask: Ask<'_>) -> Result<Shot, String> {
        let emulated = match ask.width {
            Some(w) => {
                let (view, _) = measure(webview, None)?;
                cdp(
                    webview,
                    Method::EmulationSetDeviceMetricsOverride,
                    json!({ "width": w, "height": view.inner_height.round() as i64, "deviceScaleFactor": 0, "mobile": false }),
                )?;
                // The page lays itself out at that width.
                std::thread::sleep(Duration::from_millis(300));
                true
            }
            None => false,
        };
        let shot = (|| {
            let (view, element) = measure(webview, ask.selector)?;
            let p = plan(view, element, ask.full_page)?;
            let r = p.document;
            let answer = cdp(
                webview,
                Method::PageCaptureScreenshot,
                json!({
                    "format": "jpeg",
                    "quality": JPEG_QUALITY,
                    "clip": { "x": r.x, "y": r.y, "width": r.width, "height": r.height, "scale": p.scale },
                    "captureBeyondViewport": p.full_page,
                }),
            )?;
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(answer["data"].as_str().ok_or("no capture")?)
                .map_err(|e| e.to_string())?;
            Ok(Shot {
                bytes,
                format: Format::Jpeg,
                width: r.width,
                height: r.height,
                scale: p.scale * if view.dpr > 0.0 { view.dpr } else { 1.0 },
            })
        })();
        if emulated {
            let _ = cdp(
                webview,
                Method::EmulationClearDeviceMetricsOverride,
                json!({}),
            );
        }
        shot
    }

    pub fn attach<R: Runtime>(webview: &Webview<R>, buffers: Arc<Buffers>) {
        let label = webview.label().to_string();
        let asked = webview.with_webview(move |platform| {
            // SAFETY: the controller is the live one of this WebView, used on its thread; the
            // registrations last as long as the WebView.
            let core: Result<ICoreWebView2, _> = unsafe { platform.controller().CoreWebView2() };
            let core = match core {
                Ok(core) => core,
                Err(e) => return log::error!("a browser's console is not kept: {e}"),
            };
            for event in EVENTS {
                let buffers = buffers.clone();
                let handler = DevToolsProtocolEventReceivedEventHandler::create(Box::new(
                    move |_, args: Option<ICoreWebView2DevToolsProtocolEventReceivedEventArgs>| {
                        if let Some(a) = args {
                            // A frame's own session (ESC-682) tells nothing to the browser's buffers.
                            if from_frame(&a) {
                                return Ok(());
                            }
                            let mut s = PWSTR::null();
                            // SAFETY: the event's arguments, alive for the call.
                            unsafe { a.ParameterObjectAsJson(&mut s)? };
                            if let Ok(p) = serde_json::from_str::<Value>(&take_pwstr(s)) {
                                buffers.take_cdp(event, &p);
                            }
                        }
                        Ok(())
                    },
                ));
                // SAFETY: as above.
                let listened = unsafe {
                    core.GetDevToolsProtocolEventReceiver(&HSTRING::from(event))
                        .and_then(|receiver| {
                            let mut token = 0i64;
                            receiver.add_DevToolsProtocolEventReceived(&handler, &mut token)
                        })
                };
                if let Err(e) = listened {
                    log::error!("a browser's {event} is not kept: {e}");
                }
            }
            // The frames of another site, each in its own process (ESC-682): their sessions, kept by label.
            for event in ["Target.attachedToTarget", "Target.detachedFromTarget"] {
                let label = label.clone();
                let handler = DevToolsProtocolEventReceivedEventHandler::create(Box::new(
                    move |_, args: Option<ICoreWebView2DevToolsProtocolEventReceivedEventArgs>| {
                        let Some(a) = args else { return Ok(()) };
                        if from_frame(&a) {
                            return Ok(());
                        }
                        let mut s = PWSTR::null();
                        // SAFETY: the event's arguments, alive for the call.
                        unsafe { a.ParameterObjectAsJson(&mut s)? };
                        let Ok(p) = serde_json::from_str::<Value>(&take_pwstr(s)) else {
                            return Ok(());
                        };
                        let Some(session) = p["sessionId"].as_str() else {
                            return Ok(());
                        };
                        let mut all = sessions().lock();
                        let list = all.entry(label.clone()).or_default();
                        list.retain(|(s, _)| s != session);
                        if event == "Target.attachedToTarget"
                            && p["targetInfo"]["type"].as_str() == Some("iframe")
                        {
                            if let Some(id) = p["targetInfo"]["targetId"].as_str() {
                                list.push((session.to_string(), id.to_string()));
                            }
                        }
                        Ok(())
                    },
                ));
                // SAFETY: as above.
                let listened = unsafe {
                    core.GetDevToolsProtocolEventReceiver(&HSTRING::from(event))
                        .and_then(|receiver| {
                            let mut token = 0i64;
                            receiver.add_DevToolsProtocolEventReceived(&handler, &mut token)
                        })
                };
                if let Err(e) = listened {
                    log::error!("a browser's {event} is not kept: {e}");
                }
            }
            // The domains whose events these are; the network keeps no body (its buffers at 1 byte).
            let enables = [
                (Method::TargetSetAutoAttach, auto_attach()),
                (Method::RuntimeEnable, json!({})),
                (Method::LogEnable, json!({})),
                (
                    Method::NetworkEnable,
                    json!({ "maxTotalBufferSize": 1, "maxResourceBufferSize": 1 }),
                ),
            ];
            for (method, params) in enables {
                let name = method.name();
                let handler =
                    CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |r, s| {
                        if let Err(e) = r {
                            log::error!("a browser's {name} failed: {e} {s}");
                        }
                        Ok(())
                    }));
                // SAFETY: as above.
                let sent = unsafe {
                    core.CallDevToolsProtocolMethod(
                        &HSTRING::from(name),
                        &HSTRING::from(params.to_string()),
                        &handler,
                    )
                };
                if let Err(e) = sent {
                    log::error!("a browser's {name} failed: {e}");
                }
            }
        });
        if let Err(e) = asked {
            log::error!("a browser's console is not kept: {e}");
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    /// Takes what the relay kept, every second, while the browser is open.
    pub fn attach<R: Runtime>(webview: &Webview<R>, buffers: Arc<Buffers>) {
        let webview = webview.clone();
        std::thread::spawn(move || {
            let drain = expression(&Script::Drain);
            loop {
                std::thread::sleep(POLL);
                if buffers.closed() {
                    break;
                }
                use tauri::Manager;
                if webview.app_handle().get_webview(webview.label()).is_none() {
                    break;
                }
                // A document still loading has no world yet: the next turn takes it.
                if let Ok(text) = eval(&webview, &drain) {
                    if let Ok(v) = serde_json::from_str::<Value>(&text) {
                        buffers.take_relay(&v);
                    }
                }
            }
        });
    }

    pub fn eval<R: Runtime>(webview: &Webview<R>, expression: &str) -> Result<String, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        let expression = expression.to_string();
        webview
            .with_webview(move |platform| native::eval(platform, &expression, tx))
            .map_err(|e| e.to_string())?;
        wait(rx, "the script")
    }

    pub fn capture<R: Runtime>(webview: &Webview<R>, ask: Ask<'_>) -> Result<Shot, String> {
        if ask.width.is_some() {
            return Err("another width: by the bar's widths under WebKit".into());
        }
        let (view, element) = measure(webview, ask.selector)?;
        let p = plan(view, element, ask.full_page)?;
        let (tx, rx) = std::sync::mpsc::channel();
        webview
            .with_webview(move |platform| native::snapshot(platform, p, view, tx))
            .map_err(|e| e.to_string())?;
        let bytes = wait(rx, "the capture")?;
        let dpr = if view.dpr > 0.0 { view.dpr } else { 1.0 };
        Ok(Shot {
            bytes,
            format: Format::Png,
            width: p.viewport.width,
            height: p.viewport.height,
            scale: p.scale * dpr,
        })
    }

    #[cfg(target_os = "linux")]
    mod native {
        use super::*;
        use std::sync::mpsc::Sender;
        use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};

        pub fn eval(
            platform: tauri::webview::PlatformWebview,
            expression: &str,
            tx: Sender<Result<String, String>>,
        ) {
            let view = platform.inner();
            #[allow(deprecated)]
            view.run_javascript_in_world(
                expression,
                WORLD,
                None::<&webkit2gtk::gio::Cancellable>,
                move |r| {
                    let _ = tx.send(match r {
                        Ok(r) => r
                            .js_value()
                            .map(|v| javascriptcore::ValueExt::to_str(&v).to_string())
                            .ok_or_else(|| "no value".to_string()),
                        Err(e) => Err(e.to_string()),
                    });
                },
            );
        }

        /// The view or the document by WebKit, cut to the plan's part and scaled, in PNG.
        pub fn snapshot(
            platform: tauri::webview::PlatformWebview,
            p: Plan,
            view: View,
            tx: Sender<Result<Vec<u8>, String>>,
        ) {
            let region = if p.full_page {
                SnapshotRegion::FullDocument
            } else {
                SnapshotRegion::Visible
            };
            let dpr = if view.dpr > 0.0 { view.dpr } else { 1.0 };
            platform.inner().snapshot(
                region,
                SnapshotOptions::NONE,
                None::<&webkit2gtk::gio::Cancellable>,
                move |r| {
                    let _ = tx.send(r.map_err(|e| e.to_string()).and_then(|surface| {
                        let image = cairo::ImageSurface::try_from(surface)
                            .map_err(|_| "not an image".to_string())?;
                        // The part, in the surface's pixels: the view's, or the document's from its top.
                        let part = if p.full_page {
                            Rect {
                                x: 0.0,
                                y: 0.0,
                                width: image.width() as f64,
                                height: (image.height() as f64).min(SHOT_DOC_HEIGHT * dpr),
                            }
                        } else {
                            Rect {
                                x: p.viewport.x * dpr,
                                y: p.viewport.y * dpr,
                                width: p.viewport.width * dpr,
                                height: p.viewport.height * dpr,
                            }
                        };
                        let scale = (SHOT_SIDE / part.width.max(part.height)).min(1.0);
                        let (w, h) = (
                            (part.width * scale).round().max(1.0) as i32,
                            (part.height * scale).round().max(1.0) as i32,
                        );
                        let out = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h)
                            .map_err(|e| e.to_string())?;
                        {
                            let cx = cairo::Context::new(&out).map_err(|e| e.to_string())?;
                            cx.scale(scale, scale);
                            cx.set_source_surface(&image, -part.x, -part.y)
                                .map_err(|e| e.to_string())?;
                            cx.paint().map_err(|e| e.to_string())?;
                        }
                        let mut png = Vec::new();
                        out.write_to_png(&mut png).map_err(|e| e.to_string())?;
                        Ok(png)
                    }));
                },
            );
        }
    }

    /// macOS: `WKContentWorld` and `takeSnapshotWithConfiguration:` (macOS 11 and later). Not run
    /// on a Mac yet: the first true trial is AB-5's, in the CI of macOS.
    #[cfg(target_os = "macos")]
    mod native {
        use super::*;
        use block2::RcBlock;
        use objc2::runtime::AnyObject;
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
        use objc2_foundation::{
            NSDictionary, NSError, NSNumber, NSPoint, NSRect, NSSize, NSString,
        };
        use objc2_web_kit::{WKContentWorld, WKSnapshotConfiguration, WKWebView};
        use std::sync::mpsc::Sender;

        fn webview(platform: &tauri::webview::PlatformWebview) -> Option<&WKWebView> {
            let view = platform.inner() as *const WKWebView;
            // SAFETY: wry's live WKWebView, used on the main thread (`with_webview`).
            unsafe { view.as_ref() }
        }

        pub fn eval(
            platform: tauri::webview::PlatformWebview,
            expression: &str,
            tx: Sender<Result<String, String>>,
        ) {
            let (Some(view), Some(mtm)) = (webview(&platform), MainThreadMarker::new()) else {
                let _ = tx.send(Err("no webview".into()));
                return;
            };
            let block = RcBlock::new(move |obj: *mut AnyObject, err: *mut NSError| {
                let answer = if !err.is_null() {
                    // SAFETY: WebKit's error, alive for the call.
                    Err(unsafe { &*err }.localizedDescription().to_string())
                } else if obj.is_null() {
                    Err("no value".to_string())
                } else {
                    // SAFETY: WebKit's result, alive for the call.
                    match unsafe { &*obj }.downcast_ref::<NSString>() {
                        Some(s) => Ok(s.to_string()),
                        None => Err("not a string".to_string()),
                    }
                };
                let _ = tx.send(answer);
            });
            // SAFETY: on the main thread, with a live view; the block lives as long as WebKit keeps it.
            unsafe {
                let world = WKContentWorld::worldWithName(&NSString::from_str(WORLD), mtm);
                view.evaluateJavaScript_inFrame_inContentWorld_completionHandler(
                    &NSString::from_str(expression),
                    None,
                    &world,
                    Some(&block),
                );
            }
        }

        /// The PNG of an image, by its TIFF and a bitmap of it.
        fn png(img: &NSImage) -> Option<Vec<u8>> {
            let tiff = img.TIFFRepresentation()?;
            let rep = NSBitmapImageRep::imageRepWithData(&tiff)?;
            // SAFETY: an empty dictionary of properties.
            let data = unsafe {
                rep.representationUsingType_properties(
                    NSBitmapImageFileType::PNG,
                    &NSDictionary::new(),
                )
            }?;
            Some(data.to_vec())
        }

        pub fn snapshot(
            platform: tauri::webview::PlatformWebview,
            p: Plan,
            _view: View,
            tx: Sender<Result<Vec<u8>, String>>,
        ) {
            if p.full_page {
                let _ = tx.send(Err(
                    "the whole document: not under macOS (the view alone)".into()
                ));
                return;
            }
            let (Some(view), Some(mtm)) = (webview(&platform), MainThreadMarker::new()) else {
                let _ = tx.send(Err("no webview".into()));
                return;
            };
            let block = RcBlock::new(move |img: *mut NSImage, err: *mut NSError| {
                let answer = if img.is_null() && err.is_null() {
                    Err("no capture".to_string())
                } else if img.is_null() {
                    // SAFETY: WebKit's error, alive for the call.
                    Err(unsafe { &*err }.localizedDescription().to_string())
                } else {
                    // SAFETY: WebKit's image, alive for the call.
                    png(unsafe { &*img }).ok_or_else(|| "no PNG".to_string())
                };
                let _ = tx.send(answer);
            });
            let r = p.viewport;
            // SAFETY: on the main thread, with a live view.
            unsafe {
                let config = WKSnapshotConfiguration::new(mtm);
                config.setRect(NSRect::new(
                    NSPoint::new(r.x, r.y),
                    NSSize::new(r.width, r.height),
                ));
                config.setSnapshotWidth(Some(&NSNumber::new_f64(r.width * p.scale)));
                view.takeSnapshotWithConfiguration_completionHandler(Some(&config), &block);
            }
        }
    }
}
