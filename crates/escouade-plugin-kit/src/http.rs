//! The calls of a plugin to its service: the HTTP client as Escouade's network settings make it, and a call whose
//! failure says what the service said (`HttpError`).
//!
//! [`reqwest`] is what a plugin takes its client from, on both targets: the crate itself in process, in a module the part
//! of it the plugins use, over the host's `net/fetch` (`fetch`).

#[cfg(any(test, target_family = "wasm"))]
pub mod fetch;
pub mod redirect;

#[cfg(not(target_family = "wasm"))]
pub use ::reqwest;
#[cfg(target_family = "wasm")]
pub use fetch as reqwest;

use self::reqwest::header::HeaderMap;
use crate::errors::{http_error_in, HttpError};
use crate::tr;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::time::Duration;

/// The network as Escouade's settings say it (Réglages › Avancé › Réseau), given to a plugin of the first hand when it starts:
/// `ESCOUADE_PROXY` (empty: none), `ESCOUADE_NO_PROXY`, `ESCOUADE_INSECURE_TLS` (`1`: the certificates are not
/// checked, for a proxy that inspects them without its root in the system's store). A module has no environment and needs
/// none of it: the host keeps its network.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Network {
    pub proxy: String,
    pub no_proxy: String,
    pub insecure_tls: bool,
}

impl Network {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).unwrap_or_default();
        Self {
            proxy: var("ESCOUADE_PROXY").trim().to_string(),
            no_proxy: var("ESCOUADE_NO_PROXY"),
            insecure_tls: var("ESCOUADE_INSECURE_TLS").trim() == "1",
        }
    }

    /// The client of these settings, as the core makes its own (`usage::http_client`).
    pub fn client(&self) -> Result<reqwest::Client> {
        let mut b = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(self.insecure_tls);
        if !self.proxy.is_empty() {
            let proxy = reqwest::Proxy::all(&self.proxy)?
                .no_proxy(reqwest::NoProxy::from_string(&self.no_proxy));
            b = b.proxy(proxy);
        }
        Ok(b.build()?)
    }
}

/// Sends a call to the service `name`: its JSON (null when empty) and its headers. A status other than a success is
/// an `HttpError` that says what the service said.
pub async fn send_named(name: &str, rb: reqwest::RequestBuilder) -> Result<(Value, HeaderMap)> {
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
    if !(200..300).contains(&status) {
        return Err(HttpError {
            status,
            message: http_error_in(crate::i18n::ui(), name, status, &text),
        }
        .into());
    }
    if text.trim().is_empty() {
        return Ok((Value::Null, headers));
    }
    let v = serde_json::from_str(&text).map_err(|_| {
        anyhow!(tr!(
            "{name} : réponse illisible",
            "{name}: unreadable answer"
        ))
    })?;
    Ok((v, headers))
}

/// `send_named`'s JSON.
pub async fn call(name: &str, rb: reqwest::RequestBuilder) -> Result<Value> {
    Ok(send_named(name, rb).await?.0)
}

/// `call`, and whether the answer's `Link` header points to a next page.
pub async fn call_page(name: &str, rb: reqwest::RequestBuilder) -> Result<(Value, bool)> {
    let (v, headers) = send_named(name, rb).await?;
    let next = headers
        .get_all(reqwest::header::LINK)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(','))
        .any(|link| link.contains("rel=\"next\""));
    Ok((v, next))
}

/// Percent-encoding of everything but the unreserved characters: a path segment or a query value.
pub fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The host of an address (`https://Git.Acme.dev:8443/x` → `git.acme.dev:8443`), lowercase: the name of an instance's
/// account, as the core's `forge::host_of` says it.
pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.split('/').next().unwrap_or("").to_ascii_lowercase()
}
