//! A call and its redirections. In a process reqwest follows them; in a module the host hands the 3xx to the plugin, so
//! that a fetch to its `Location` is judged like any other (docs/PLUGINS.md §8.4.1): this does here what reqwest did, on
//! both targets, so that a module answers as the process did. A renamed repository, or a host that moved to https, leads
//! the next call to its new address; a host that is not the service's gets neither its credentials nor a body: a call
//! that still has one stops at the redirection that would leave, and gives it back.

use escouade_plugin_kit::errors::{http_error_in, HttpError};
use escouade_plugin_kit::http::{host_of, reqwest};
use escouade_plugin_kit::tr;

use anyhow::{anyhow, Result};
use reqwest::header::HeaderMap;
use reqwest::Method;
use serde_json::Value;
use std::future::Future;

/// How many in a row: reqwest's limit.
const MAX: usize = 10;

/// What a call answered, whatever its status.
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) headers: HeaderMap,
    pub(crate) text: String,
}

/// A header, its name in any case.
fn location(headers: &HeaderMap) -> Option<&str> {
    headers.get("location").and_then(|v| v.to_str().ok())
}

/// `location`, against the address that answered it.
fn absolute(from: &str, location: &str) -> Option<String> {
    let location = location.trim();
    if location.starts_with("http://") || location.starts_with("https://") {
        return Some(location.to_string());
    }
    let (scheme, rest) = from.split_once("://")?;
    if location.is_empty() {
        return None;
    }
    if let Some(net) = location.strip_prefix("//") {
        return Some(format!("{scheme}://{net}"));
    }
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    let authority = path.split('/').next().unwrap_or(path);
    if location.starts_with('/') {
        return Some(format!("{scheme}://{authority}{location}"));
    }
    let folder = path.rfind('/').map_or(authority, |i| &path[..i]);
    Some(format!("{scheme}://{folder}/{location}"))
}

/// Where a redirection leads.
#[derive(Debug, PartialEq)]
struct Hop {
    url: String,
    /// 301, 302 and 303 end in a GET without a body; 307 and 308 send the same call again, on the service alone.
    as_get: bool,
    /// Still the service: never an other host, nor from https to http.
    own: bool,
}

fn hop(status: u16, from: &str, location: Option<&str>) -> Option<Hop> {
    if !matches!(status, 301 | 302 | 303 | 307 | 308) {
        return None;
    }
    let url = absolute(from, location?)?;
    let secure = |u: &str| u.starts_with("https://");
    Some(Hop {
        own: host_of(from) == host_of(&url) && (secure(&url) || !secure(from)),
        as_get: status < 307,
        url,
    })
}

/// Sends a call once and reads what it answered; only a failure to reach the service `name` is an error.
pub(crate) async fn reply(name: &str, rb: reqwest::RequestBuilder) -> Result<Reply> {
    let resp = rb.send().await.map_err(|e| {
        let e = anyhow::Error::from(e.without_url());
        anyhow!(tr!(
            "{name} injoignable : {e:#}",
            "{name} can’t be reached: {e:#}"
        ))
    })?;
    let status = resp.status().as_u16();
    let headers = resp.headers().clone();
    let text = resp.text().await.unwrap_or_default();
    Ok(Reply {
        status,
        headers,
        text,
    })
}

/// The call `method url`, sent by `send` with its address, whether that is still the service's, and `body`, and again
/// wherever it is redirected: its last answer, whatever its status. A body never leaves the service: the redirection
/// that would take it elsewhere, or from https to http, is that last answer, and nothing goes to the other host.
// `Method` is `Copy` in a module, not in a process.
#[allow(clippy::clone_on_copy)]
pub(crate) async fn follow<B: Copy, F: Future<Output = Result<Reply>>>(
    name: &str,
    method: Method,
    url: String,
    body: Option<B>,
    mut send: impl FnMut(Method, String, bool, Option<B>) -> F,
) -> Result<Reply> {
    let (mut method, mut url, mut body, mut own) = (method, url, body, true);
    for _ in 0..MAX {
        let answer = send(method.clone(), url.clone(), own, body).await?;
        let Some(next) = hop(answer.status, &url, location(&answer.headers)) else {
            return Ok(answer);
        };
        own = own && next.own;
        if !own && body.is_some() {
            return Ok(answer);
        }
        if next.as_get {
            method = Method::GET;
            body = None;
        }
        url = next.url;
    }
    Err(anyhow!(tr!(
        "{name} : trop de redirections",
        "{name}: too many redirects"
    )))
}

/// `http::send_named`, its redirections followed: its JSON (null when empty) and its headers, `make` giving the request
/// for an address, whether that is still the service's. A status other than a success is an `HttpError`.
pub(crate) async fn send_named<B: Copy>(
    name: &str,
    method: Method,
    url: String,
    body: Option<B>,
    make: impl Fn(Method, &str, bool, Option<B>) -> reqwest::RequestBuilder,
) -> Result<(Value, HeaderMap)> {
    let make = &make;
    let answer = follow(name, method, url, body, |m, u, own, b| async move {
        reply(name, make(m, &u, own, b)).await
    })
    .await?;
    let answer = success(name, answer)?;
    if answer.text.trim().is_empty() {
        return Ok((Value::Null, answer.headers));
    }
    let v = serde_json::from_str(&answer.text).map_err(|_| {
        anyhow!(tr!(
            "{name} : réponse illisible",
            "{name}: unreadable answer"
        ))
    })?;
    Ok((v, answer.headers))
}

/// An answer other than a success is an error that says what the service said.
pub(crate) fn success(name: &str, answer: Reply) -> Result<Reply> {
    if (200..300).contains(&answer.status) {
        return Ok(answer);
    }
    Err(HttpError {
        status: answer.status,
        message: http_error_in(
            escouade_plugin_kit::i18n::ui(),
            name,
            answer.status,
            &answer.text,
        ),
    }
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_location_is_read_against_the_address_that_answered() {
        let from = "https://git.acme.dev:8443/api/v1/repos/a/b?x=1";
        assert_eq!(
            absolute(from, "https://else.dev/x").as_deref(),
            Some("https://else.dev/x")
        );
        assert_eq!(
            absolute(from, "//else.dev/x").as_deref(),
            Some("https://else.dev/x")
        );
        assert_eq!(
            absolute(from, "/repositories/7").as_deref(),
            Some("https://git.acme.dev:8443/repositories/7")
        );
        assert_eq!(
            absolute(from, "c?y=2").as_deref(),
            Some("https://git.acme.dev:8443/api/v1/repos/a/c?y=2")
        );
        assert_eq!(absolute(from, "  "), None);
    }

    #[test]
    fn only_a_redirection_with_a_location_leads_somewhere() {
        for status in [200, 202, 304, 404] {
            assert_eq!(hop(status, "https://a.dev/x", Some("/y")), None);
        }
        assert_eq!(hop(301, "https://a.dev/x", None), None);
    }

    #[test]
    fn a_write_stays_one_on_a_307_and_a_308_alone() {
        for (status, as_get) in [
            (301, true),
            (302, true),
            (303, true),
            (307, false),
            (308, false),
        ] {
            assert_eq!(
                hop(status, "https://a.dev/x", Some("/y")).unwrap().as_get,
                as_get,
                "{status}"
            );
        }
    }

    #[test]
    fn the_credentials_stay_on_the_service_and_on_https() {
        let own = |from, to| hop(302, from, Some(to)).unwrap().own;
        assert!(own("https://a.dev/x", "https://a.dev/y"));
        assert!(own("http://a.dev/x", "https://a.dev/y"));
        assert!(!own("https://a.dev/x", "http://a.dev/y"));
        assert!(!own("https://a.dev/x", "https://b.dev/y"));
        assert!(!own("https://a.dev:1/x", "https://a.dev:2/y"));
    }

    #[test]
    fn the_host_is_never_read_in_a_path() {
        assert!(
            !hop(302, "https://a.dev/x", Some("https://b.dev/a.dev"))
                .unwrap()
                .own
        );
    }

    /// Plays `follow` on the given answers, each a status and its `Location`: the calls it sent, the address, whether it
    /// was still the service's, and whether a body went with it, then the status it gave back.
    fn played(
        first: &str,
        body: Option<&'static str>,
        answers: &[(u16, &str)],
    ) -> (Vec<(String, bool, bool)>, u16) {
        let sent = std::cell::RefCell::new(Vec::new());
        let at = std::cell::Cell::new(0);
        let send = |_: Method, url: String, own: bool, body: Option<&'static str>| {
            sent.borrow_mut().push((url, own, body.is_some()));
            let (status, to) = answers[at.replace(at.get() + 1)];
            let mut headers = HeaderMap::new();
            if !to.is_empty() {
                headers.insert("location", to.parse().unwrap());
            }
            std::future::ready(Ok(Reply {
                status,
                headers,
                text: String::new(),
            }))
        };
        let run = follow("Service", Method::POST, first.to_string(), body, send);
        let mut run = std::pin::pin!(run);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready(answer) = run.as_mut().poll(&mut cx) else {
            panic!("nothing here waits");
        };
        (sent.take(), answer.unwrap().status)
    }

    #[test]
    fn a_body_does_not_leave_the_service() {
        for status in [301, 302, 303, 307, 308] {
            let (sent, last) = played(
                "https://a.dev/x",
                Some("{}"),
                &[(status, "https://b.blob.core.windows.net/y"), (200, "")],
            );
            assert_eq!(
                sent,
                [("https://a.dev/x".to_string(), true, true)],
                "{status}"
            );
            assert_eq!(last, status);
        }
    }

    #[test]
    fn a_body_does_not_go_down_from_https_to_http_either() {
        let (sent, last) = played(
            "https://a.dev/x",
            Some("{}"),
            &[(308, "http://a.dev/x"), (200, "")],
        );
        assert_eq!(sent.len(), 1);
        assert_eq!(last, 308);
    }

    #[test]
    fn a_body_stops_at_the_hop_that_leaves_after_those_that_stayed() {
        let (sent, last) = played(
            "https://a.dev/x",
            Some("{}"),
            &[(307, "/y"), (307, "https://b.dev/z"), (200, "")],
        );
        assert_eq!(
            sent,
            [
                ("https://a.dev/x".to_string(), true, true),
                ("https://a.dev/y".to_string(), true, true),
            ]
        );
        assert_eq!(last, 307);
    }

    #[test]
    fn a_body_follows_on_the_same_host() {
        let (sent, last) = played("https://a.dev/x", Some("{}"), &[(307, "/y"), (200, "")]);
        assert_eq!(
            sent,
            [
                ("https://a.dev/x".to_string(), true, true),
                ("https://a.dev/y".to_string(), true, true),
            ]
        );
        assert_eq!(last, 200);
    }

    #[test]
    fn without_a_body_a_call_goes_to_the_other_host_without_its_credentials() {
        let (sent, last) = played(
            "https://a.dev/x",
            None,
            &[(307, "https://b.dev/y"), (200, "")],
        );
        assert_eq!(
            sent,
            [
                ("https://a.dev/x".to_string(), true, false),
                ("https://b.dev/y".to_string(), false, false),
            ]
        );
        assert_eq!(last, 200);
    }

    #[test]
    fn a_body_dropped_on_the_service_lets_the_get_leave_it() {
        let (sent, last) = played(
            "https://a.dev/x",
            Some("{}"),
            &[(303, "/y"), (302, "https://b.dev/z"), (200, "")],
        );
        assert_eq!(
            sent,
            [
                ("https://a.dev/x".to_string(), true, true),
                ("https://a.dev/y".to_string(), true, false),
                ("https://b.dev/z".to_string(), false, false),
            ]
        );
        assert_eq!(last, 200);
    }
}
