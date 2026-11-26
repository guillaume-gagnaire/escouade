//! An installed extension's files, read-only, with their MIME type: what the core's protocol (`escouade-ext:`) answers
//! and what the page's command reads. Only a file of an extension of the store (installed, so checked), inside its
//! folder: no `..`, no absolute path, no link, nothing else of the disk. Open VSX's own `unpkg` serves JavaScript as
//! `text/plain` with `nosniff` (a worker refuses it) and refuses big files (§3.1 of the study): here each file has its
//! type, `nosniff` stays.
//!
//! The protocol's address: `escouade-ext://localhost/<publisher.name>/<path>` (macOS, Linux), or
//! `http://escouade-ext.localhost/<publisher.name>/<path>` (Windows): the path alone is read.

use crate::allowed::Layer;
use crate::store::Store;
use crate::{Code, Refusal, Result};

/// The scheme of the core's protocol.
pub const SCHEME: &str = "escouade-ext";

/// The most a file served may weigh.
pub const MAX_FILE: u64 = 64 << 20;

/// Whether `dir` is a folder name the store makes (`publisher.name-version`, in lower case).
pub fn is_dir_name(dir: &str) -> bool {
    !dir.is_empty()
        && dir.len() <= 200
        && !dir.starts_with('.')
        && !dir.contains("..")
        && dir
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
}

/// The MIME type of a file, by its extension.
pub fn mime_of(path: &str) -> &'static str {
    let ext = path
        .rsplit('/')
        .next()
        .and_then(|n| n.rsplit_once('.'))
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "js" | "mjs" | "cjs" => "text/javascript; charset=utf-8",
        "json" | "code-snippets" | "code-workspace" | "map" => "application/json; charset=utf-8",
        "tmlanguage" | "tmtheme" | "plist" | "xml" => "application/xml; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "html" | "htm" => "text/html; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

fn not_found(what: &str) -> Refusal {
    Refusal::new(
        Code::NotFound,
        tr!("« {what} » n’est pas servi", "“{what}” is not served"),
    )
}

fn forbidden(what: &str) -> Refusal {
    Refusal::new(
        Code::Trapped,
        tr!(
            "« {what} » est hors du dossier de l’extension",
            "“{what}” is outside the extension’s folder"
        ),
    )
}

fn turned_off(key: &str) -> Refusal {
    Refusal::new(
        Code::Off,
        tr!(
            "L’extension « {key} » est coupée : ses fichiers ne sont pas servis.",
            "The extension “{key}” is turned off: its files are not served."
        ),
    )
}

/// The folder of `key`, when it may be served (ESC-648, SR4-07): installed, on, and allowed by `layers` (the allowed
/// list as the core judges it). A turned off or refused extension is never served, whatever the page does with it.
pub fn served_dir(store: &Store, layers: &[Layer], key: &str) -> Result<std::path::PathBuf> {
    let Some(e) = store.get(key) else {
        return Err(not_found(key));
    };
    if !e.enabled {
        return Err(turned_off(&e.id));
    }
    let id = crate::ExtId::parse(&e.id)?;
    crate::allowed::check(layers, &id, Some((&e.version, e.pre_release)))?;
    store.dir_of(key).ok_or_else(|| not_found(key))
}

/// The file `rel` of the installed extension `key`: its bytes and its type. Never of an extension turned off.
pub fn read(store: &Store, key: &str, rel: &str) -> Result<(Vec<u8>, &'static str)> {
    read_allowed(store, &[], key, rel)
}

/// The file `rel` of `key`, when `layers` allow it too (`served_dir`).
pub fn read_allowed(
    store: &Store,
    layers: &[Layer],
    key: &str,
    rel: &str,
) -> Result<(Vec<u8>, &'static str)> {
    if crate::unpack::bad_name(rel).is_some() {
        return Err(forbidden(rel));
    }
    let dir = served_dir(store, layers, key)?;
    let path = escouade_base::paths::contained(&dir, rel).map_err(|_| forbidden(rel))?;
    // Not followed: a link is never served, even one that would stay inside.
    let meta = std::fs::symlink_metadata(&path).map_err(|_| not_found(rel))?;
    if !meta.file_type().is_file() {
        return Err(not_found(rel));
    }
    if meta.len() > MAX_FILE {
        return Err(not_found(rel));
    }
    let bytes = std::fs::read(&path).map_err(|_| not_found(rel))?;
    Ok((bytes, mime_of(rel)))
}

/// The answer of the protocol to `method` on `path` (`/<publisher.name>/<file>`, percent-encoded).
pub fn respond(store: &Store, method: &str, path: &str) -> http::Response<Vec<u8>> {
    respond_allowed(store, &[], method, path)
}

/// The answer of the protocol, the extensions `layers` refuse (and those turned off) answered 403.
pub fn respond_allowed(
    store: &Store,
    layers: &[Layer],
    method: &str,
    path: &str,
) -> http::Response<Vec<u8>> {
    let answer = |status: u16, mime: &str, body: Vec<u8>| {
        http::Response::builder()
            .status(status)
            .header(http::header::CONTENT_TYPE, mime)
            .header("X-Content-Type-Options", "nosniff")
            // A page of an extension sends no Referer where it links to or loads from (ESC-654).
            .header(http::header::REFERRER_POLICY, "no-referrer")
            .header(http::header::CACHE_CONTROL, "no-cache")
            .body(body)
            .expect("a response of fixed headers")
    };
    let text = |status: u16, s: String| answer(status, "text/plain; charset=utf-8", s.into_bytes());
    if method != "GET" && method != "HEAD" {
        return text(405, "read-only".into());
    }
    let decoded = percent_encoding::percent_decode_str(path.trim_start_matches('/'))
        .decode_utf8()
        .map(|s| s.into_owned());
    let Ok(decoded) = decoded else {
        return text(400, "bad path".into());
    };
    let (key, rel) = decoded.split_once('/').unwrap_or((&decoded, ""));
    match read_allowed(store, layers, key, rel) {
        Ok((bytes, mime)) => answer(200, mime, if method == "HEAD" { Vec::new() } else { bytes }),
        Err(r) if matches!(r.code, Code::Trapped | Code::Off | Code::NotAllowed) => {
            text(403, r.message)
        }
        Err(r) => text(404, r.message),
    }
}
