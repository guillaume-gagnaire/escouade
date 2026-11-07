//! The errors of a service that the core tells apart: an HTTP error (401 says « reconnect the account »), a refusal that
//! asking again would not change. They cross the protocol as codes (`escouade_plugin_protocol::code`) and come back as
//! these very types in the core (`plugins::adapters`), so that a client in a plugin and one in the core fail alike.

use crate::i18n::Lang;
use crate::tr_in;
use serde_json::Value;

/// A service's answer other than a success: its status, and what `http_error_in` says of it (the error's text).
#[derive(Debug)]
pub struct HttpError {
    pub status: u16,
    pub message: String,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HttpError {}

/// A refusal that asking again would not change (no transition leads from where the issue is to the state).
#[derive(Debug)]
pub struct Refused(pub String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refused {}

/// What `name` answering `status` with `body` means, in `lang`.
pub fn http_error_in(lang: Lang, name: &str, status: u16, body: &str) -> String {
    let tail = said(body).map(|m| format!(" — {m}")).unwrap_or_default();
    match status {
        401 => tr_in!(
            lang,
            "{name} refuse ces identifiants (401){tail}",
            "{name} refuses these credentials (401){tail}"
        ),
        403 => tr_in!(
            lang,
            "{name} refuse l'accès (403){tail}",
            "{name} denies access (403){tail}"
        ),
        404 => tr_in!(
            lang,
            "{name} : introuvable (404){tail}",
            "{name}: not found (404){tail}"
        ),
        429 => tr_in!(
            lang,
            "{name} limite les requêtes (429) : réessaie dans un moment",
            "{name} limits the requests (429): try again in a moment"
        ),
        _ => tr_in!(
            lang,
            "{name} : erreur {status}{tail}",
            "{name}: error {status}{tail}"
        ),
    }
}

/// The message of an error's body: Jira's `errorMessages` or `errors`, the first of GraphQL's `errors`, GitHub's
/// `message`, ClickUp's `err`, or a text's first line; none for a page of HTML.
pub fn said(body: &str) -> Option<String> {
    let body = body.trim();
    let msg = match serde_json::from_str::<Value>(body) {
        Ok(Value::String(s)) => s,
        Ok(v) => v["errorMessages"][0]
            .as_str()
            .map(str::to_string)
            .or_else(|| {
                v["errors"]
                    .as_object()
                    .and_then(|o| o.values().find_map(|e| e.as_str().map(str::to_string)))
            })
            .or_else(|| v["errors"][0]["message"].as_str().map(str::to_string))
            .or_else(|| v["message"].as_str().map(str::to_string))
            .or_else(|| v["error"].as_str().map(str::to_string))
            // Bitbucket's `{"type": "error", "error": {"message": …}}`.
            .or_else(|| v["error"]["message"].as_str().map(str::to_string))
            .or_else(|| v["err"].as_str().map(str::to_string))?,
        Err(_) if body.starts_with('<') => return None,
        Err(_) => body.lines().next()?.to_string(),
    };
    let msg = msg.trim();
    (!msg.is_empty()).then(|| first_line(msg))
}

/// The first line that says something, cut to 200 bytes.
pub fn first_line(e: &str) -> String {
    let line = e
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| crate::tr!("erreur inconnue", "unknown error"));
    truncate(&line, 200)
}

/// At most `max` bytes, an ellipsis after when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}
