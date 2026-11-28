//! The page of the browser's client, served on the remote API's port (BC-1, ESC-704, `docs/REMOTE.md` « Le client
//! navigateur »): `escouade-server --browser` alone (`RemoteApi::serve_page`); off, none of this runs.
//!
//! - **What it serves**: `GET /` (`index.html`) and `GET /<name>` of each other file the binary carries (`escouade-web`,
//!   made from `dist-web/` by `npm run build:web`: `assets/`, and what Vite copies of `public/`, as the window has it:
//!   What's up ?, the tour's images, the logo), `HEAD` of the same; nothing of the disk, nothing else. A name is looked
//!   up as it is given: no decoding, no `..`, nothing hidden (`/assets/%2e%2e/x` is no name of it).
//! - **Its headers, on each answer**: the window's content security policy (`src-tauri/tauri.conf.json`), adapted:
//!   Tauri's schemes out, `style-src 'self' 'nonce-…'` with a nonce drawn for this answer and put on
//!   `<style id="escouade-style-nonce">` (`lib/style-nonce.ts` shares it), `connect-src` with the page's `ws(s)://`;
//!   `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Cross-Origin-Opener-Policy` and
//!   `Cross-Origin-Resource-Policy: same-origin`, a `Permissions-Policy` that closes the microphone, the camera and the
//!   geolocation; `assets/` immutable (their names are drawn from their content), the rest in `no-cache` with an
//!   `ETag` (a `304` when the client has it already), `index.html` without one (its nonce changes each time).
//! - **The mark**: `<meta name="escouade-served" content="escouade-server">` in `index.html`, which tells the page it is
//!   served (BC-2): never Tauri's absence.

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use sha2::Digest as _;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Read as _;
use std::sync::LazyLock;

/// The mark that tells the page it is served by `escouade-server` (BC-2 reads it).
pub const SERVED_MARK: &str = r#"<meta name="escouade-served" content="escouade-server" />"#;
/// The `<style>` of `index.html` that carries the nonce (`lib/style-nonce.ts`, `NONCE_STYLE_ID`).
const NONCE_STYLE: &str = r#"<style id="escouade-style-nonce">"#;
/// How long a file of `assets/` is kept: a year, never asked again (its name changes with its content).
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
/// The folder whose files are named from their content (Vite's `assets/`): kept, never asked again.
const HASHED: &str = "assets/";
/// What a page of this origin may never ask (the browser's client has no voice, no camera, no place).
pub const PERMISSIONS: &str = "microphone=(), camera=(), geolocation=()";

/// The window's policy (`src-tauri/tauri.conf.json`), as Tauri gives it.
static WINDOW_CSP: LazyLock<String> = LazyLock::new(|| {
    let conf: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap_or_default();
    conf["app"]["security"]["csp"]
        .as_str()
        .unwrap_or_default()
        .to_string()
});

/// The page's policy for one answer: the window's, Tauri's schemes out (`ipc:`, `asset:`, their `*.localhost`, the
/// development's `ws://127.0.0.1:*`); `style-src` takes `nonce`; `connect-src` takes `socket` (the page's
/// `ws(s)://<Host>`); `script-src` takes the hashes of the inline scripts of an HTML of `assets/` (as Tauri does).
pub fn policy(nonce: &str, socket: &str, script_hashes: &[String]) -> String {
    let tauri = |v: &str| {
        matches!(
            v,
            "ipc:" | "asset:" | "http://ipc.localhost" | "http://asset.localhost"
        ) || v.starts_with("ws://127.0.0.1")
    };
    let mut out = Vec::new();
    for directive in WINDOW_CSP.split(';') {
        let mut words = directive.split_whitespace();
        let Some(name) = words.next() else { continue };
        let mut values: Vec<String> = words.filter(|v| !tauri(v)).map(String::from).collect();
        match name {
            "style-src" => values.push(format!("'nonce-{nonce}'")),
            "connect-src" if !socket.is_empty() => values.push(socket.to_string()),
            "script-src" => values.extend(script_hashes.iter().map(|h| format!("'sha256-{h}'"))),
            _ => {}
        }
        out.push(format!("{name} {}", values.join(" ")));
    }
    out.join("; ")
}

/// A nonce for one answer: 16 random bytes, in base64.
fn nonce() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the system's random generator");
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// An HTML file of the page, ready for a nonce: its text, where each `<style>` takes it, where the mark goes
/// (`index.html`), and the hashes of its inline scripts.
struct Html {
    text: String,
    styles: Vec<usize>,
    mark: Option<usize>,
    scripts: Vec<String>,
}

impl Html {
    fn new(text: String, marked: bool) -> Self {
        let scripts = inline_scripts(&text)
            .into_iter()
            .map(|s| base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(s)))
            .collect();
        let styles = style_points(&text);
        let mark = if marked { text.find(NONCE_STYLE) } else { None };
        Self {
            text,
            styles,
            mark,
            scripts,
        }
    }

    /// Its text with `nonce` on each `<style>` (never in a script's text), and the mark before the nonce's `<style>`.
    fn with(&self, nonce: &str) -> String {
        let attribute = format!(r#" nonce="{nonce}""#);
        let mut out = String::with_capacity(self.text.len() + 256);
        let mut at = 0;
        let mut points: Vec<(usize, &str)> = self
            .styles
            .iter()
            .map(|&p| (p, attribute.as_str()))
            .collect();
        let mark = format!("{SERVED_MARK}\n    ");
        if let Some(m) = self.mark {
            points.push((m, mark.as_str()));
        }
        points.sort_by_key(|(p, _)| *p);
        for (point, insert) in points {
            out.push_str(&self.text[at..point]);
            out.push_str(insert);
            at = point;
        }
        out.push_str(&self.text[at..]);
        out
    }
}

/// Where a `<style` element of `html` takes its nonce (just after its name), outside the scripts' text and the comments.
fn style_points(html: &str) -> Vec<usize> {
    let mut points = Vec::new();
    let mut i = 0;
    while let Some(off) = html[i..].find('<') {
        i += off;
        let rest = &html[i..];
        // A comment says what it likes (index.html names its `<style>`): nothing in it is an element.
        if rest.starts_with("<!--") {
            match rest.find("-->") {
                Some(end) => {
                    i += end + "-->".len();
                    continue;
                }
                None => break,
            }
        }
        if rest.starts_with("<script") {
            match rest.find("</script>") {
                Some(end) => {
                    i += end + "</script>".len();
                    continue;
                }
                None => break,
            }
        }
        if rest.starts_with("<style")
            && matches!(
                rest.as_bytes().get(6),
                Some(b' ' | b'>' | b'\t' | b'\n' | b'\r')
            )
        {
            points.push(i + "<style".len());
        }
        i += 1;
    }
    points
}

/// The contents of the `<script>` elements of `html` without a `src`.
fn inline_scripts(html: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        let Some(end_tag) = rest[start..].find('>') else {
            break;
        };
        let tag = &rest[start..start + end_tag];
        let body = &rest[start + end_tag + 1..];
        let Some(close) = body.find("</script>") else {
            break;
        };
        if !tag.contains(" src=") {
            out.push(&body[..close]);
        }
        rest = &body[close..];
    }
    out
}

/// A file of `assets/`: its bytes compressed by gzip, its type, and the HTML it is once opened (an HTML of `assets/`).
struct Asset {
    gzip: Cow<'static, [u8]>,
    kind: &'static str,
    html: Option<Html>,
    /// Named from its content (`assets/`): immutable; else revalidated by its `etag`.
    hashed: bool,
    etag: String,
}

/// A name of the page it serves as it is: segments of printable ASCII, none empty nor hidden (no `.`, `..`), no `\`,
/// `%`, `?` or `#`.
pub fn servable(name: &str) -> bool {
    !name.is_empty()
        && name.split('/').all(|segment| {
            !segment.is_empty()
                && !segment.starts_with('.')
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_graphic() && !matches!(b, b'\\' | b'%' | b'?' | b'#'))
        })
}

/// Whether `if_none_match` (an `If-None-Match`) names `etag`, or any.
fn matches_etag(if_none_match: &str, etag: &str) -> bool {
    if_none_match
        .split(',')
        .map(|t| t.trim().trim_start_matches("W/"))
        .any(|t| t == "*" || t == etag)
}

/// The page, as the server serves it.
pub struct Page {
    index: Html,
    assets: HashMap<String, Asset>,
    /// What the log says of it.
    pub summary: String,
}

impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Page({})", self.summary)
    }
}

/// An answer of the page: its status, its headers, its body.
pub struct Answer {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

impl Answer {
    /// Its bytes on the wire (`HTTP/1.1`, its length, the connection closed after it); a `HEAD`'s without its body.
    pub fn bytes(&self, head_only: bool) -> Vec<u8> {
        let reason = match self.status {
            200 => "OK",
            304 => "Not Modified",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            431 => "Request Header Fields Too Large",
            _ => "",
        };
        let mut out = format!("HTTP/1.1 {} {reason}\r\n", self.status);
        for (name, value) in &self.headers {
            out.push_str(&format!("{name}: {value}\r\n"));
        }
        // A 304 has no body, and says no length (it would be the file's).
        if self.status != 304 {
            out.push_str(&format!("Content-Length: {}\r\n", self.body.len()));
        }
        out.push_str("Connection: close\r\n\r\n");
        let mut out = out.into_bytes();
        if !head_only {
            out.extend_from_slice(&self.body);
        }
        out
    }

    /// The value of its header `name` (any case).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// The type of a file, by its extension.
fn kind_of(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "txt" | "tmLanguage" | "code-snippets" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn gunzip(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes).read_to_end(&mut out)?;
    Ok(out)
}

impl Page {
    /// The page of `files` (`index.html`, and the other files by their path in `dist-web/`, each compressed by gzip).
    /// Refused: no `index.html`, or one without the `<style>` of the nonce (the styles would not apply), or a name it
    /// would not serve as it is ([`servable`]).
    pub fn new(files: impl IntoIterator<Item = (String, Cow<'static, [u8]>)>) -> Result<Self> {
        let mut index = None;
        let mut assets = HashMap::new();
        let (mut count, mut bytes) = (0usize, 0usize);
        for (name, gzip) in files {
            count += 1;
            bytes += gzip.len();
            if name == "index.html" {
                let text = String::from_utf8(gunzip(&gzip)?).context("index.html")?;
                if text.matches(NONCE_STYLE).count() != 1 {
                    bail!("index.html has no single {NONCE_STYLE}");
                }
                index = Some(Html::new(text, true));
                continue;
            }
            if !servable(&name) {
                bail!("{name}: not a name it serves");
            }
            let kind = kind_of(&name);
            let html = if name.ends_with(".html") {
                Some(Html::new(
                    String::from_utf8(gunzip(&gzip)?).with_context(|| name.clone())?,
                    false,
                ))
            } else {
                None
            };
            let etag = format!("\"{}\"", hex::encode(&sha2::Sha256::digest(&gzip)[..16]));
            let hashed = name.starts_with(HASHED);
            assets.insert(
                name,
                Asset {
                    gzip,
                    kind,
                    html,
                    hashed,
                    etag,
                },
            );
        }
        let index = index.context("the page has no index.html")?;
        Ok(Self {
            index,
            assets,
            summary: format!("{count} files, {} kB compressed", bytes / 1000),
        })
    }

    /// The page `escouade-web` carries (`(name, gzip)`): none when it is empty (a build without `ESCOUADE_WEB_DIST`).
    pub fn embedded(files: &'static [(&'static str, &'static [u8])]) -> Result<Option<Self>> {
        if files.is_empty() {
            return Ok(None);
        }
        Self::new(
            files
                .iter()
                .map(|(name, gzip)| (name.to_string(), Cow::Borrowed(*gzip))),
        )
        .map(Some)
    }

    /// The answer to `GET` (or `HEAD`) of `path` (its query aside), for a page reached at `socket`
    /// (`ws://127.0.0.1:8765`), the client taking gzip or not, and having the version `if_none_match` names or not.
    pub fn answer(
        &self,
        path: &str,
        socket: &str,
        gzip: bool,
        if_none_match: Option<&str>,
    ) -> Answer {
        let path = path.split(['?', '#']).next().unwrap_or("");
        let nonce = nonce();
        if path == "/" {
            let body = self.index.with(&nonce).into_bytes();
            return Self::with_headers(
                200,
                "text/html; charset=utf-8",
                "no-cache",
                &nonce,
                socket,
                &self.index.scripts,
                body,
            );
        }
        // index.html has one address, `/`.
        let asset = path
            .strip_prefix('/')
            .filter(|name| *name != "index.html")
            .and_then(|name| self.assets.get(name));
        let Some(asset) = asset else {
            return Self::refusal(404, "not found", socket);
        };
        let cache = if asset.hashed { IMMUTABLE } else { "no-cache" };
        if let Some(html) = &asset.html {
            let body = html.with(&nonce).into_bytes();
            return Self::with_headers(200, asset.kind, cache, &nonce, socket, &html.scripts, body);
        }
        if if_none_match.is_some_and(|tags| matches_etag(tags, &asset.etag)) {
            let mut a = Self::with_headers(304, asset.kind, cache, &nonce, socket, &[], Vec::new());
            a.headers.push(("ETag", asset.etag.clone()));
            a.headers.push(("Vary", "Accept-Encoding".into()));
            return a;
        }
        let mut answer = if gzip {
            let mut a = Self::with_headers(
                200,
                asset.kind,
                cache,
                &nonce,
                socket,
                &[],
                asset.gzip.to_vec(),
            );
            a.headers.push(("Content-Encoding", "gzip".into()));
            a
        } else {
            match gunzip(&asset.gzip) {
                Ok(body) => Self::with_headers(200, asset.kind, cache, &nonce, socket, &[], body),
                Err(_) => return Self::refusal(404, "not found", socket),
            }
        };
        answer.headers.push(("ETag", asset.etag.clone()));
        answer.headers.push(("Vary", "Accept-Encoding".into()));
        answer
    }

    /// A refusal (`status`), with the page's headers, kept nowhere.
    pub fn refusal(status: u16, why: &str, socket: &str) -> Answer {
        Self::with_headers(
            status,
            "text/plain; charset=utf-8",
            "no-store",
            &nonce(),
            socket,
            &[],
            why.as_bytes().to_vec(),
        )
    }

    fn with_headers(
        status: u16,
        kind: &str,
        cache: &str,
        nonce: &str,
        socket: &str,
        scripts: &[String],
        body: Vec<u8>,
    ) -> Answer {
        Answer {
            status,
            headers: vec![
                ("Content-Type", kind.to_string()),
                ("Cache-Control", cache.to_string()),
                ("Content-Security-Policy", policy(nonce, socket, scripts)),
                ("X-Content-Type-Options", "nosniff".into()),
                ("Referrer-Policy", "no-referrer".into()),
                ("Cross-Origin-Opener-Policy", "same-origin".into()),
                ("Cross-Origin-Resource-Policy", "same-origin".into()),
                ("Permissions-Policy", PERMISSIONS.into()),
            ],
            body,
        }
    }
}

/// Whether `accept` (an `Accept-Encoding`) takes gzip: named, with no `q=0`.
pub fn takes_gzip(accept: &str) -> bool {
    accept.split(',').any(|part| {
        let mut bits = part.split(';').map(str::trim);
        let name = bits.next().unwrap_or("");
        let refused = bits.any(|p| {
            p.strip_prefix("q=")
                .and_then(|q| q.parse::<f32>().ok())
                .is_some_and(|q| q == 0.0)
        });
        name.eq_ignore_ascii_case("gzip") && !refused
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_policy_is_the_windows_without_tauri_with_the_nonce_and_the_socket() {
        let csp = policy("N0nce==", "ws://127.0.0.1:8765", &[]);
        let directives: HashMap<&str, Vec<&str>> = csp
            .split("; ")
            .map(|d| {
                let mut w = d.split(' ');
                (w.next().unwrap(), w.collect())
            })
            .collect();
        assert_eq!(directives["style-src"], ["'self'", "'nonce-N0nce=='"]);
        assert_eq!(directives["style-src-attr"], ["'unsafe-inline'"]);
        assert_eq!(directives["script-src"], ["'self'", "'wasm-unsafe-eval'"]);
        assert_eq!(
            directives["connect-src"],
            ["'self'", "blob:", "data:", "ws://127.0.0.1:8765"]
        );
        assert_eq!(directives["default-src"], ["'self'"]);
        for name in ["object-src", "base-uri", "form-action", "frame-ancestors"] {
            assert_eq!(directives[name], ["'none'"], "{name}");
        }
        assert!(!csp.contains("ipc") && !csp.contains("asset") && !csp.contains("localhost"));
        // Every directive of the window is there.
        assert_eq!(directives.len(), WINDOW_CSP.split(';').count());
    }

    #[test]
    fn gzip_is_taken_unless_refused() {
        assert!(takes_gzip("gzip, deflate, br"));
        assert!(takes_gzip("br;q=1.0, GZIP;q=0.5"));
        assert!(!takes_gzip("gzip;q=0"));
        assert!(!takes_gzip("identity"));
        assert!(!takes_gzip(""));
    }

    #[test]
    fn the_inline_scripts_are_hashed_and_the_others_not() {
        let html = r#"<script src="/a.js"></script><script>one()</script><script type="module">two()</script>"#;
        assert_eq!(inline_scripts(html), ["one()", "two()"]);
    }

    #[test]
    fn the_nonce_goes_on_each_style_and_never_in_a_script() {
        let html = r#"<head><!-- its `<style>` --><style id="escouade-style-nonce"></style><script>const s = "<style>";</script><style>a{}</style><styles></head>"#;
        let out = Html::new(html.to_string(), true).with("N");
        assert_eq!(
            out,
            format!(
                r#"<head><!-- its `<style>` -->{SERVED_MARK}
    <style nonce="N" id="escouade-style-nonce"></style><script>const s = "<style>";</script><style nonce="N">a{{}}</style><styles></head>"#
            )
        );
    }
}
