//! The part of reqwest the plugins of Escouade use (ESC-330, docs/PLUGINS.md §8.4.1), over the host's `net/fetch`.

use serde::Serialize;
use serde_json::{json, Map, Value};
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;
use std::time::Duration;

pub type Answer = Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>;
type Transport = fn(Value) -> Answer;
static TRANSPORT: OnceLock<Transport> = OnceLock::new();

/// How a request reaches the host, given once by the kit.
pub fn transport(f: Transport) {
    let _ = TRANSPORT.set(f);
}

#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub fn without_url(self) -> Self {
        self
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

pub mod header {
    pub const LINK: &str = "link";

    #[derive(Debug, Clone, Default)]
    pub struct HeaderMap(pub(crate) Vec<(String, HeaderValue)>);

    #[derive(Debug, Clone)]
    pub struct HeaderValue(pub(crate) String);

    impl HeaderValue {
        pub fn to_str(&self) -> Result<&str, std::fmt::Error> {
            Ok(&self.0)
        }
    }

    impl HeaderMap {
        pub fn get(&self, name: &str) -> Option<&HeaderValue> {
            self.0
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v)
        }
        pub fn get_all(&self, name: &str) -> GetAll<'_> {
            GetAll(self, name.to_string())
        }
    }

    pub struct GetAll<'a>(&'a HeaderMap, String);

    impl GetAll<'_> {
        pub fn iter(&self) -> impl Iterator<Item = &HeaderValue> + '_ {
            self.0
                 .0
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case(&self.1))
                .map(|(_, v)| v)
        }
    }
}

pub struct NoProxy;
impl NoProxy {
    pub fn from_string(_: &str) -> Option<NoProxy> {
        None
    }
}

/// Réglages › Réseau are the host's: nothing of a proxy reaches a module.
pub struct Proxy;
impl Proxy {
    pub fn all(_: &str) -> Result<Proxy, Error> {
        Ok(Proxy)
    }
    pub fn no_proxy(self, _: Option<NoProxy>) -> Proxy {
        self
    }
}

#[derive(Default)]
pub struct ClientBuilder;
impl ClientBuilder {
    pub fn timeout(self, _: Duration) -> Self {
        self
    }
    pub fn danger_accept_invalid_certs(self, _: bool) -> Self {
        self
    }
    pub fn proxy(self, _: Proxy) -> Self {
        self
    }
    pub fn build(self) -> Result<Client, Error> {
        Ok(Client)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Client;

impl Client {
    pub fn new() -> Self {
        Client
    }
    pub fn builder() -> ClientBuilder {
        ClientBuilder
    }
    fn request(&self, method: &str, url: String) -> RequestBuilder {
        RequestBuilder {
            method: method.to_string(),
            url,
            headers: Map::new(),
            body: None,
        }
    }
    pub fn get(&self, url: impl Into<String>) -> RequestBuilder {
        self.request("GET", url.into())
    }
    pub fn post(&self, url: impl Into<String>) -> RequestBuilder {
        self.request("POST", url.into())
    }
}

pub struct RequestBuilder {
    method: String,
    url: String,
    headers: Map<String, Value>,
    body: Option<String>,
}

impl RequestBuilder {
    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.headers
            .insert(name.as_ref().to_string(), json!(value.as_ref()));
        self
    }
    pub fn json<T: Serialize + ?Sized>(mut self, body: &T) -> Self {
        self.body = serde_json::to_string(body).ok();
        self.headers
            .insert("Content-Type".into(), json!("application/json"));
        self
    }
    pub async fn send(self) -> Result<Response, Error> {
        let Some(fetch) = TRANSPORT.get() else {
            return Err(Error("no transport".into()));
        };
        let v = fetch(json!({
            "url": self.url, "method": self.method, "headers": self.headers, "body": self.body,
        }))
        .await
        .map_err(Error)?;
        let headers = v["headers"].as_object().map_or_else(Vec::new, |h| {
            h.iter()
                .map(|(k, v)| {
                    let value = v.as_str().unwrap_or_default().to_string();
                    (k.clone(), header::HeaderValue(value))
                })
                .collect()
        });
        Ok(Response {
            status: StatusCode(v["status"].as_u64().unwrap_or(0) as u16),
            headers: header::HeaderMap(headers),
            body: v["body"].as_str().unwrap_or_default().to_string(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(u16);
impl StatusCode {
    pub fn as_u16(self) -> u16 {
        self.0
    }
}

pub struct Response {
    status: StatusCode,
    headers: header::HeaderMap,
    body: String,
}

impl Response {
    pub fn status(&self) -> StatusCode {
        self.status
    }
    pub fn headers(&self) -> &header::HeaderMap {
        &self.headers
    }
    pub async fn text(self) -> Result<String, Error> {
        Ok(self.body)
    }
}
