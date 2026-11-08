//! The part of reqwest the plugins of Escouade use, over the host's `net/fetch` (docs/PLUGINS.md §8.4.1): what
//! `escouade_plugin_kit::http::reqwest` is in a module, where reqwest does not compile, so that a plugin's client is the
//! same source on both targets. What `net/fetch` cannot give (a stream) is not here: `crate::host` reads `net/stream`.
//!
//! As reqwest does, `send` follows redirections, by the rules of `super::redirect`. A header other than those that say
//! what is wanted may hold a key, whatever its name, so none of them leaves the service.
//!
//! The network is the host's: a proxy, the certificates and the time a call may take are settled there, so what says them
//! here is accepted and has no effect.

use super::redirect::{hop, MAX};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::future::Future;
use std::time::Duration;

/// The headers a call keeps once it left its service.
const HARMLESS: [&str; 3] = ["accept", "accept-language", "user-agent"];

/// A call that failed: what the host said.
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    fn of(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn without_url(self) -> Self {
        self
    }

    /// Whether it ran out of time, as `net/fetch` says it.
    pub fn is_timeout(&self) -> bool {
        self.message.contains("no answer within")
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub mod header {
    pub const LINK: &str = "link";

    #[derive(Debug, Clone, Default)]
    pub struct HeaderMap(pub(super) Vec<(String, HeaderValue)>);

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct HeaderValue(pub(super) String);

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
            let name = &self.1;
            self.0
                 .0
                .iter()
                .filter(move |(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v)
        }
    }
}

/// What `Proxy::no_proxy` takes: nothing of it reaches a module.
pub struct NoProxy;

impl NoProxy {
    pub fn from_string(_: &str) -> Option<NoProxy> {
        None
    }
}

/// Réglages › Avancé › Réseau are the host's.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Method(&'static str);

impl Method {
    pub const GET: Method = Method("GET");
    pub const POST: Method = Method("POST");
    pub const PUT: Method = Method("PUT");
    pub const PATCH: Method = Method("PATCH");
    pub const DELETE: Method = Method("DELETE");
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

    pub fn request(&self, method: Method, url: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder {
            method,
            url: url.as_ref().to_string(),
            headers: Map::new(),
            body: None,
        }
    }

    pub fn get(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    pub fn post(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    pub fn put(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    pub fn patch(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    pub fn delete(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }
}

#[derive(Debug, Clone)]
pub struct RequestBuilder {
    method: Method,
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

    pub fn bearer_auth(self, token: impl std::fmt::Display) -> Self {
        self.header("Authorization", format!("Bearer {token}"))
    }

    pub fn basic_auth(
        self,
        user: impl std::fmt::Display,
        password: Option<impl std::fmt::Display>,
    ) -> Self {
        use base64::Engine;
        let pair = match password {
            Some(p) => format!("{user}:{p}"),
            None => format!("{user}:"),
        };
        let pair = base64::engine::general_purpose::STANDARD.encode(pair);
        self.header("Authorization", format!("Basic {pair}"))
    }

    pub fn json<T: Serialize + ?Sized>(mut self, body: &T) -> Self {
        self.body = serde_json::to_string(body).ok();
        let typed = self
            .headers
            .keys()
            .any(|k| k.eq_ignore_ascii_case("content-type"));
        if typed {
            self
        } else {
            self.header("Content-Type", "application/json")
        }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// The pairs of `query`, percent-encoded, after the address's.
    pub fn query<T: Serialize + ?Sized>(mut self, query: &T) -> Self {
        let pairs = serde_json::to_value(query).unwrap_or(Value::Null);
        let text = |v: &Value| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let pairs: Vec<(String, String)> = match pairs {
            Value::Object(o) => o.iter().map(|(k, v)| (k.clone(), text(v))).collect(),
            Value::Array(a) => a.iter().map(|p| (text(&p[0]), text(&p[1]))).collect(),
            _ => Vec::new(),
        };
        for (k, v) in pairs {
            let joint = if self.url.contains('?') { '&' } else { '?' };
            let (k, v) = (super::encode(&k), super::encode(&v));
            self.url.push_str(&format!("{joint}{k}={v}"));
        }
        self
    }

    pub fn try_clone(&self) -> Option<Self> {
        Some(self.clone())
    }

    fn params(&self) -> Value {
        let mut p = json!({ "url": self.url, "method": self.method.0, "headers": self.headers });
        if let Some(body) = &self.body {
            p["body"] = json!(body);
        }
        p
    }

    /// The call, once: a 3xx is its answer.
    pub async fn once(self) -> Result<Response, Error> {
        Self::fetched(self.params()).await
    }

    async fn fetched(params: Value) -> Result<Response, Error> {
        let cx = crate::serve::reactor::cx().ok_or_else(|| Error::of("no host"))?;
        let v = cx
            .request("net/fetch", params)
            .await
            .map_err(|e| Error::of(e.message))?;
        Ok(Response::of(&v))
    }

    /// The call, and again wherever it is redirected: its last answer.
    pub async fn send(self) -> Result<Response, Error> {
        self.walk(Self::fetched).await
    }

    /// `send`, each call made by `fetch`. A body never leaves the service: the redirection that would take it elsewhere,
    /// or from https to http, is the last answer.
    async fn walk<F, A>(mut self, mut fetch: F) -> Result<Response, Error>
    where
        F: FnMut(Value) -> A,
        A: Future<Output = Result<Response, Error>>,
    {
        let mut own = true;
        for _ in 0..MAX {
            let answer = fetch(self.params()).await?;
            let location = answer
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok());
            let Some(next) = hop(answer.status().as_u16(), &self.url, location) else {
                return Ok(answer);
            };
            own = own && next.own;
            if !own && self.body.is_some() {
                return Ok(answer);
            }
            if next.as_get {
                self.method = Method::GET;
                self.body = None;
                self.headers
                    .retain(|k, _| !k.eq_ignore_ascii_case("content-type"));
            }
            if !own {
                let kept = |k: &str| HARMLESS.iter().any(|h| k.eq_ignore_ascii_case(h));
                self.headers.retain(|k, _| kept(k));
            }
            self.url = next.url;
        }
        Err(Error::of(format!(
            "{}: too many redirects",
            super::host_of(&self.url)
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(u16);

impl StatusCode {
    pub fn as_u16(self) -> u16 {
        self.0
    }

    pub fn is_success(self) -> bool {
        (200..300).contains(&self.0)
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct Response {
    status: StatusCode,
    headers: header::HeaderMap,
    body: String,
}

impl Response {
    /// What `net/fetch` answered.
    fn of(v: &Value) -> Self {
        let headers = v["headers"].as_object().map_or_else(Vec::new, |h| {
            h.iter()
                .map(|(k, v)| {
                    let value = v.as_str().unwrap_or_default().to_string();
                    (k.clone(), header::HeaderValue(value))
                })
                .collect()
        });
        use base64::Engine;
        let body = match (v["body"].as_str(), v["bodyBase64"].as_str()) {
            (Some(text), _) => text.to_string(),
            (None, Some(bytes)) => {
                let bytes = base64::engine::general_purpose::STANDARD.decode(bytes);
                String::from_utf8_lossy(&bytes.unwrap_or_default()).into_owned()
            }
            (None, None) => String::new(),
        };
        Self {
            status: StatusCode(u16::try_from(v["status"].as_u64().unwrap_or(0)).unwrap_or(0)),
            headers: header::HeaderMap(headers),
            body,
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &header::HeaderMap {
        &self.headers
    }

    pub async fn text(self) -> Result<String, Error> {
        Ok(self.body)
    }

    pub async fn json<T: DeserializeOwned>(self) -> Result<T, Error> {
        serde_json::from_str(&self.body).map_err(|e| Error::of(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(rb: RequestBuilder) -> Value {
        rb.params()
    }

    #[test]
    fn a_request_says_what_net_fetch_reads() {
        let p = params(
            Client::new()
                .request(Method::PATCH, "https://x/y")
                .header("Accept", "application/json")
                .body("raw"),
        );
        assert_eq!(p["method"], "PATCH");
        assert_eq!(p["url"], "https://x/y");
        assert_eq!(p["headers"]["Accept"], "application/json");
        assert_eq!(p["body"], "raw");
        assert!(params(Client::new().get("https://x")).get("body").is_none());
    }

    #[test]
    fn the_authorizations_read_as_reqwest_writes_them() {
        let auth = |rb: RequestBuilder| params(rb).clone()["headers"]["Authorization"].clone();
        assert_eq!(auth(Client::new().get("u").bearer_auth("t")), "Bearer t");
        assert_eq!(
            auth(Client::new().get("u").basic_auth("ada", Some("pw"))),
            "Basic YWRhOnB3"
        );
        assert_eq!(
            auth(Client::new().get("u").basic_auth("ada", None::<&str>)),
            "Basic YWRhOg=="
        );
    }

    #[test]
    fn a_content_type_of_the_plugin_stays_over_json() {
        let p = params(
            Client::new()
                .post("u")
                .header("content-type", "application/vnd+json")
                .json(&json!({ "a": 1 })),
        );
        assert_eq!(p["headers"]["content-type"], "application/vnd+json");
        assert!(p["headers"].get("Content-Type").is_none());
    }

    #[test]
    fn a_query_goes_after_the_address_encoded() {
        let q: &[(&str, String)] = &[("a b", "1&2".to_string()), ("c", "é".to_string())];
        let p = params(Client::new().get("https://x/y").query(q));
        assert_eq!(p["url"], "https://x/y?a%20b=1%262&c=%C3%A9");
        let p = params(Client::new().get("https://x/y?z=1").query(q));
        assert!(p["url"]
            .as_str()
            .unwrap()
            .starts_with("https://x/y?z=1&a%20b="));
    }

    #[test]
    fn a_request_clones_for_a_second_try() {
        let rb = Client::new().post("u").json(&json!({ "a": 1 }));
        assert_eq!(params(rb.try_clone().unwrap()), params(rb));
    }

    #[test]
    fn an_answer_reads_as_a_response_of_reqwest() {
        let r = Response::of(&json!({
            "status": 403,
            "headers": { "Link": "<u>; rel=\"next\"", "X-Other": "1" },
            "body": "{\"a\":1}",
        }));
        assert_eq!(r.status().as_u16(), 403);
        assert!(!r.status().is_success());
        assert_eq!(r.status().to_string(), "403");
        let link = r.headers().get(header::LINK).unwrap();
        assert_eq!(link.to_str().unwrap(), "<u>; rel=\"next\"");
        assert_eq!(r.headers().get_all("x-other").iter().count(), 1);
        assert_eq!(r.headers().clone().get("nothing"), None);
    }

    #[test]
    fn what_is_not_utf8_reads_with_its_replacement() {
        let r = Response::of(&json!({ "status": 200, "bodyBase64": "Yf8=" }));
        assert_eq!(r.body, "a\u{fffd}");
    }

    /// Plays `send` on the given answers, each a status and its `Location`: what `net/fetch` was asked each time, then
    /// the status it gave back.
    fn walked(rb: RequestBuilder, answers: &[(u16, &str)]) -> (Vec<Value>, Result<u16, Error>) {
        let sent = std::cell::RefCell::new(Vec::new());
        let at = std::cell::Cell::new(0);
        let fetch = |params: Value| {
            sent.borrow_mut().push(params);
            let (status, to) = answers[at.replace(at.get() + 1)];
            let headers = json!({ "Location": to });
            std::future::ready(Ok(Response::of(
                &json!({ "status": status, "headers": headers, "body": "" }),
            )))
        };
        let run = rb.walk(fetch);
        let mut run = std::pin::pin!(run);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready(answer) = run.as_mut().poll(&mut cx) else {
            panic!("nothing here waits");
        };
        (sent.take(), answer.map(|r| r.status().as_u16()))
    }

    #[test]
    fn a_read_follows_and_leaves_its_credentials_behind() {
        let rb = Client::new()
            .get("https://a.dev/x")
            .bearer_auth("k")
            .header("PRIVATE-TOKEN", "k")
            .header("Accept", "text/plain");
        let (sent, last) = walked(rb, &[(301, "/y"), (302, "https://b.dev/z"), (200, "")]);
        assert_eq!(last.unwrap(), 200);
        assert_eq!(sent[1]["url"], "https://a.dev/y");
        assert_eq!(
            sent[1]["headers"]["Authorization"], "Bearer k",
            "the service still"
        );
        assert_eq!(sent[2]["url"], "https://b.dev/z");
        assert_eq!(sent[2]["headers"], json!({ "Accept": "text/plain" }));
    }

    #[test]
    fn a_write_turns_into_a_get_after_a_303_and_stays_one_after_a_307() {
        let rb = || {
            Client::new()
                .post("https://a.dev/x")
                .json(&json!({ "a": 1 }))
        };
        let (sent, _) = walked(rb(), &[(303, "/y"), (200, "")]);
        assert_eq!(sent[1]["method"], "GET");
        assert!(sent[1].get("body").is_none());
        assert_eq!(sent[1]["headers"], json!({}));
        let (sent, _) = walked(rb(), &[(307, "/y"), (200, "")]);
        assert_eq!(sent[1]["method"], "POST");
        assert_eq!(sent[1]["body"], r#"{"a":1}"#);
    }

    #[test]
    fn a_body_does_not_leave_the_service() {
        let rb = || {
            Client::new()
                .post("https://a.dev/x")
                .json(&json!({ "a": 1 }))
        };
        for (status, to) in [(307, "https://b.dev/y"), (308, "http://a.dev/x")] {
            let (sent, last) = walked(rb(), &[(status, to), (200, "")]);
            assert_eq!(sent.len(), 1, "{to}");
            assert_eq!(last.unwrap(), status);
        }
    }

    #[test]
    fn ten_hops_and_no_more() {
        let (sent, last) = walked(Client::new().get("https://a.dev/x"), &[(302, "/y"); 11]);
        assert_eq!(sent.len(), MAX);
        assert!(last.unwrap_err().to_string().contains("too many redirects"));
    }

    #[test]
    fn a_3xx_without_a_location_is_an_answer() {
        let (sent, last) = walked(Client::new().get("https://a.dev/x"), &[(304, "")]);
        assert_eq!((sent.len(), last.unwrap()), (1, 304));
    }

    #[test]
    fn a_timeout_of_the_host_reads_as_one() {
        assert!(Error::of("api.linear.app: no answer within 10 s").is_timeout());
        assert!(!Error::of("api.linear.app: refused").is_timeout());
    }

    #[test]
    fn what_the_network_settings_say_has_no_effect_here() {
        let proxy = Proxy::all("http://p").unwrap();
        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .danger_accept_invalid_certs(true)
            .proxy(proxy.no_proxy(NoProxy::from_string("x")))
            .build();
        assert!(client.is_ok());
    }
}
