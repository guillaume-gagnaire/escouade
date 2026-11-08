//! What the app learns when a request of the updater fails, and how it says it.
//!
//! The plugin that checks and downloads the updates tells little of a failure: « update endpoint
//! did not respond with a successful status code », then « Could not fetch a valid release JSON »,
//! never the HTTP code nor who answered. A request that failed is therefore made again by the
//! app itself (`probe`), with the same proxy, TLS settings, headers and `Accept`, to read what
//! answered: the code, the domain, whether a proxy stands in the way and what it calls itself.
//!
//! The app never gets around a company's security rule: it does not pass for a browser, it says
//! who it is (`user_agent`), who refused, and sends the user to the browser instead. A proxy that
//! answers, whatever it says (a 403, a 407, a tunnel turned down), is kept; the only proxy gone
//! around is one that is not there (`Failure::proxy_unreachable`).

use crate::i18n::Lang;
use crate::model::Settings;
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE};
use reqwest::{StatusCode, Url};
use std::fmt;
use std::time::Duration;

/// Where the release page is, when the link that failed does not tell.
pub const RELEASES_URL: &str = "https://github.com/guillaume-gagnaire/escouade/releases/latest";

/// How long the probe may take, and how long of it may go to reaching the server.
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const PROBE_CONNECT: Duration = Duration::from_secs(10);

/// How much of an error page is read, and how much of its text is kept.
const PAGE_BYTES: usize = 8 * 1024;
const PAGE_CHARS: usize = 160;

/// Which way a request goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// What the settings ask for: through their proxy, or the system's when they have none.
    Proxy,
    /// Without any proxy.
    Direct,
}

/// The proxy a request goes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via<'a> {
    Nothing,
    /// The system's (the variables of the environment, the settings of the system), which the
    /// plugin and reqwest follow when they are given none.
    System,
    Proxy(&'a Url),
}

/// The network settings the updates follow.
#[derive(Debug, Clone, Default)]
pub struct Network {
    /// The proxy of the settings: none when the setting is empty.
    pub proxy: Option<Url>,
    /// With no proxy in the settings, the system's is followed (the plugin does, unless told not
    /// to, and the app's other requests as well). Off, nothing is: the tests of the updates never
    /// reach for the proxy of the machine they run on.
    pub system_proxy: bool,
    /// TLS verification is off.
    pub insecure_tls: bool,
}

impl Network {
    /// The settings as they are now. An address that is not one is an error the user can read.
    pub fn from_settings(settings: &Settings) -> Result<Self> {
        let address = settings.proxy_url.trim();
        let proxy = if address.is_empty() {
            None
        } else {
            Some(address.parse::<Url>().map_err(|e| {
                anyhow!(tr!(
                    "adresse du proxy invalide : {e}",
                    "invalid proxy address: {e}"
                ))
            })?)
        };
        Ok(Self {
            proxy,
            system_proxy: true,
            insecure_tls: settings.insecure_tls,
        })
    }

    /// What a request that goes `route` goes through.
    pub fn via(&self, route: Route) -> Via<'_> {
        match (route, &self.proxy) {
            (Route::Direct, _) => Via::Nothing,
            (Route::Proxy, Some(proxy)) => Via::Proxy(proxy),
            (Route::Proxy, None) if self.system_proxy => Via::System,
            (Route::Proxy, None) => Via::Nothing,
        }
    }
}

/// What the app says of itself to the servers it asks and to the proxies in between:
/// `Escouade/1.8.0 (Windows; x86_64)`. A proxy's rules can then name the app, which never passes
/// for anything else.
pub fn user_agent() -> String {
    let system = if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    format!(
        "Escouade/{} ({system}; {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH
    )
}

/// What a request was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    Check,
    Download,
}

/// Where a request that went through a proxy stopped, when it never got an answer from the server
/// and the proxy is to blame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stuck {
    /// The proxy could not be reached (the connection refused or timed out, its name not found), or
    /// it never answered the tunnel: it is not there, so the way around it harms no rule.
    ProxyDown,
    /// The proxy answered the tunnel (`CONNECT`) with a refusal: its rule, kept.
    TunnelRefused,
    /// The proxy answered the tunnel with a 407: it wants a user name and a password.
    TunnelAuth,
}

/// A request of the updater that failed, and what is known of why.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub what: What,
    /// What was asked.
    pub url: String,
    /// The HTTP code of the answer; none when there was no answer, or when a successful one could
    /// not be read.
    pub status: Option<u16>,
    /// Something answered, whatever it said: told apart from the network failing, which has no
    /// code either.
    pub answered: bool,
    /// The domain that answered (after the redirections), or the one that could not be reached.
    pub host: String,
    /// `host:port` of the proxy the request went through, when it did.
    pub proxy: Option<String>,
    /// What the answer says of itself when a proxy sent it: `Zscaler/6.2`.
    pub agent: Option<String>,
    /// The start of the text of an error page.
    pub page: Option<String>,
    /// The error itself: the network's, or the plugin's.
    pub detail: String,
    /// What the proxy did when the request got no answer through it.
    pub stuck: Option<Stuck>,
    /// The endpoints of latest.json: the origins, besides GitHub's, the browser may be sent to.
    pub trusted: Vec<Url>,
    /// The same request without the proxy, tried and failed too.
    pub direct: Option<Box<Failure>>,
}

impl Failure {
    /// It may pass by itself: a network that comes back, a link that is renewed, a server that
    /// recovers, a release whose files are still being added. A 403 is one of those: the links of
    /// a release expire, and a proxy's block is lifted or not. A password the proxy asks for (a
    /// 407, its tunnel's too) does not come by itself.
    pub fn transient(&self) -> bool {
        match self.status {
            None => self.stuck != Some(Stuck::TunnelAuth),
            Some(code) => matches!(code, 403 | 404 | 408 | 425 | 429) || code >= 500,
        }
    }

    /// The proxy of the settings is not there: the connection to it was refused or timed out, its
    /// name is unknown, or it never answered the tunnel. The one case where the request is made
    /// again without it: a proxy that answers, whatever it says, is a company's rule, and is kept.
    pub fn proxy_unreachable(&self) -> bool {
        self.proxy.is_some() && self.stuck == Some(Stuck::ProxyDown)
    }

    /// The proxy answered and turned the request down: a 403 or a 407, or a tunnel refused. It is
    /// told in the words of a company's proxy, and never gone around.
    pub fn refused_by_proxy(&self) -> bool {
        self.proxy.is_some()
            && (matches!(self.stuck, Some(Stuck::TunnelRefused | Stuck::TunnelAuth))
                || matches!(self.status, Some(403 | 407)))
    }

    /// The proxy wants a user name and a password (a 407, its tunnel's too).
    fn asks_for_credentials(&self) -> bool {
        self.proxy.is_some() && (self.status == Some(407) || self.stuck == Some(Stuck::TunnelAuth))
    }

    /// The cause, in `lang`: the window puts what was being done before it.
    pub fn describe(&self, lang: Lang) -> String {
        let cause = self.cause(lang);
        match &self.direct {
            Some(direct) => {
                let direct = direct.describe(lang);
                tr_in!(
                    lang,
                    "{cause} ; sans proxy : {direct}",
                    "{cause}; without the proxy: {direct}"
                )
            }
            None => cause,
        }
    }

    fn cause(&self, lang: Lang) -> String {
        // A link is told without its query, whoever made this failure: it may be a signed one.
        let host = if self.host.is_empty() {
            without_queries(&self.url)
        } else {
            self.host.clone()
        };
        let detail = without_queries(&self.detail);
        let proxy = self.proxy.as_deref();
        if let Some(p) = proxy {
            // The proxy asks for credentials, or turned a tunnel down: its words, not a guess at
            // a block. Nothing else can be said of an answer the app never saw.
            if self.asks_for_credentials() {
                let status = status_text(407);
                return tr_in!(
                    lang,
                    "le proxy {p} demande une authentification ({status}) : mets l'identifiant et le mot de passe dans l'adresse du proxy, dans Réglages › Avancé › Réseau",
                    "the proxy {p} asks for authentication ({status}): put the user name and password in the proxy address, in Settings › Advanced › Network"
                );
            }
            if self.stuck == Some(Stuck::TunnelRefused) {
                let browser = match self.what {
                    What::Download => tr_in!(
                        lang,
                        " Télécharge l'installateur dans le navigateur.",
                        " Download the installer in the browser."
                    ),
                    What::Check => String::new(),
                };
                return tr_in!(
                    lang,
                    "le proxy {p} a refusé d'ouvrir la connexion à {host} (tunnel CONNECT refusé) : une règle de ton entreprise le bloque peut-être.{browser}",
                    "the proxy {p} refused to open the connection to {host} (CONNECT tunnel refused): a rule of your company may be blocking it.{browser}"
                );
            }
        }
        let Some(code) = self.status else {
            return if self.answered {
                tr_in!(
                    lang,
                    "réponse inattendue de {host} : {detail}",
                    "unexpected answer from {host}: {detail}"
                )
            } else {
                let through = match proxy {
                    Some(p) => tr_in!(lang, " par le proxy {p}", " through the proxy {p}"),
                    None => String::new(),
                };
                tr_in!(
                    lang,
                    "connexion impossible à {host}{through} ({detail})",
                    "couldn’t reach {host}{through} ({detail})"
                )
            };
        };
        let status = status_text(code);
        let from = match proxy {
            Some(p) => tr_in!(
                lang,
                "{status} de {host}, par le proxy {p}",
                "{status} from {host}, through the proxy {p}"
            ),
            None => tr_in!(lang, "{status} de {host}", "{status} from {host}"),
        };
        if self.what == What::Download && self.refused_by_proxy() {
            if let Some(agent) = &self.agent {
                return tr_in!(
                    lang,
                    "le proxy de ton entreprise ({agent}) a refusé l'installateur ({from}). Télécharge-le dans le navigateur : il faudra peut-être confirmer l'avertissement du proxy.",
                    "your company’s proxy ({agent}) refused the installer ({from}). Download it in the browser: you may have to confirm the proxy’s warning."
                );
            }
        }
        let said = match (&self.agent, &self.page) {
            (Some(agent), Some(page)) => {
                tr_in!(lang, " ({agent} : « {page} »)", " ({agent}: “{page}”)")
            }
            (Some(agent), None) => format!(" ({agent})"),
            (None, Some(page)) => tr_in!(lang, " (« {page} »)", " (“{page}”)"),
            (None, None) => String::new(),
        };
        let blocked = if matches!(code, 403 | 407) {
            tr_in!(
                lang,
                " : bloqué par le réseau ou un proxy ?",
                ": blocked by the network or a proxy?"
            )
        } else {
            String::new()
        };
        format!("{from}{said}{blocked}")
    }

    /// The page « Télécharger dans le navigateur » opens: that of the release the link is of.
    pub fn page_url(&self) -> String {
        release_page(&self.url, &self.trusted)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe(crate::i18n::ui()))
    }
}

impl std::error::Error for Failure {}

/// `403 Forbidden`, or the code alone when the registry has no name for it.
fn status_text(code: u16) -> String {
    match StatusCode::from_u16(code)
        .ok()
        .and_then(|s| s.canonical_reason())
    {
        Some(reason) => format!("{code} {reason}"),
        None => code.to_string(),
    }
}

/// The page of the release `url` is a file of (`…/releases/download/<tag>/<file>`), when the
/// browser may be sent there: that is GitHub's (in https), or the origin (scheme, host and port)
/// of one of the `trusted` endpoints of latest.json. The link comes from a latest.json that a
/// proxy which does not verify certificates, or a mirror, could have altered: it must not send
/// the browser just anywhere. The latest release of the app otherwise, as when the link is not
/// one of a release's files.
pub fn release_page(url: &str, trusted: &[Url]) -> String {
    const DOWNLOAD: &str = "/releases/download/";
    let Ok(link) = Url::parse(url) else {
        return RELEASES_URL.into();
    };
    let github = link.scheme() == "https"
        && link.host_str() == Some("github.com")
        && link.port_or_known_default() == Some(443);
    let allowed = github
        || trusted
            .iter()
            .any(|endpoint| endpoint.origin() == link.origin());
    // A login in front of the domain is never a release's.
    if !allowed || !link.username().is_empty() || link.password().is_some() {
        return RELEASES_URL.into();
    }
    let path = link.path();
    if let Some(at) = path.find(DOWNLOAD) {
        let (prefix, rest) = (&path[..at], &path[at + DOWNLOAD.len()..]);
        let tag = rest.split('/').next().unwrap_or_default();
        if !tag.is_empty() && rest[tag.len()..].starts_with('/') {
            let mut page = link.clone();
            page.set_path(&format!("{prefix}/releases/tag/{tag}"));
            // The query of a link (a signature) and its fragment are not the page's.
            page.set_query(None);
            page.set_fragment(None);
            return page.to_string();
        }
    }
    RELEASES_URL.into()
}

/// Where a request that went through a proxy stopped, from the error of the network and whether
/// it was a timeout: the proxy is not there (a connection refused or timed out, a name not found,
/// a tunnel that got no word), it turned the tunnel down, or it wants credentials for it.
/// Everything else (a certificate refused, a connection reset after the answer began) is none of
/// these. reqwest tells the tunnel's answer in the text of its error (hyper-util's `TunnelError`)
/// and nothing of its code.
pub fn stuck_of(error: &(dyn std::error::Error + 'static), timed_out: bool) -> Option<Stuck> {
    use std::io::ErrorKind::{ConnectionRefused, HostUnreachable, NetworkUnreachable, TimedOut};
    let mut down = timed_out;
    let mut source = Some(error);
    while let Some(cause) = source {
        let text = cause.to_string().to_lowercase();
        if text.contains("tunnel") {
            if text.contains("unsuccessful") {
                return Some(Stuck::TunnelRefused);
            }
            if text.contains("authoriz") || text.contains("authentic") {
                return Some(Stuck::TunnelAuth);
            }
            down |= [
                "unexpected end of file",
                "io error",
                "failed to create underlying connection",
            ]
            .iter()
            .any(|silent| text.contains(silent));
        }
        down |= text.contains("dns error");
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            down |= matches!(
                io.kind(),
                ConnectionRefused | TimedOut | NetworkUnreachable | HostUnreachable
            );
        }
        source = cause.source();
    }
    down.then_some(Stuck::ProxyDown)
}

/// `text` without the query and the fragment of the links in it: GitHub redirects a download to
/// a link signed in its query (`?sig=…&se=…`), which is a key to the file, and reqwest says the
/// link of a request that failed. A link goes to the next blank, quote, parenthesis or angle
/// bracket; a text with no link is left as it is.
pub fn without_queries(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = link_start(rest) {
        out.push_str(&rest[..start]);
        let link = &rest[start..];
        let end = link
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '(' | ')' | '<' | '>'))
            .unwrap_or(link.len());
        let (url, after) = link.split_at(end);
        out.push_str(url.split(['?', '#']).next().unwrap_or(url));
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Where the first link of `text` begins (`http://` or `https://`, in any case).
fn link_start(text: &str) -> Option<usize> {
    // ASCII lower case keeps every offset.
    let lower = text.to_ascii_lowercase();
    ["http://", "https://"]
        .iter()
        .filter_map(|scheme| lower.find(scheme))
        .min()
}

/// An error and its causes in one line: « error sending request: client error (Connect): … ».
/// The links in it are without their query: the plugin's errors say the link they failed on,
/// signed or not (`without_queries`).
pub fn chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let part = cause.to_string();
        // Some errors repeat their cause in their own text.
        if !text.contains(&part) {
            text.push_str(": ");
            text.push_str(&part);
        }
        source = cause.source();
    }
    without_queries(&text)
}

/// `text` kept to `PAGE_CHARS` characters, an ellipsis included when it was cut.
fn cut(text: &str) -> String {
    if text.chars().count() <= PAGE_CHARS {
        return text.into();
    }
    let mut kept: String = text.chars().take(PAGE_CHARS - 1).collect();
    kept.push('…');
    kept
}

/// The text of a page of HTML or plain text: no tags, blanks merged, 160 characters at most
/// (an ellipsis included when it was cut).
pub fn page_text(html: &str) -> String {
    let html = strip_blocks(&strip_blocks(html, "script"), "style");
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // A `<` opens a tag when a letter, a slash or a bang follows: « a < b » is text. A tag
            // is a break between words.
            '<' if !in_tag
                && chars
                    .peek()
                    .is_some_and(|n| n.is_ascii_alphabetic() || matches!(n, '/' | '!')) =>
            {
                in_tag = true;
                text.push(' ');
            }
            '>' if in_tag => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    let decoded = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        // Last, so that « &amp;lt; » stays « &lt; ».
        .replace("&amp;", "&");
    let merged = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    // A page may name the link that was blocked, signed or not.
    cut(&without_queries(&merged))
}

/// `html` without the `<tag>…</tag>` blocks (a script, a style sheet: code, not words), the ones
/// that never end included.
fn strip_blocks(html: &str, tag: &str) -> String {
    // ASCII lower case keeps every offset.
    let lower = html.to_ascii_lowercase();
    let (open, close) = (format!("<{tag}"), format!("</{tag}>"));
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    while let Some(start) = lower[at..].find(&open).map(|i| at + i) {
        out.push_str(&html[at..start]);
        match lower[start..].find(&close) {
            Some(end) => at = start + end + close.len(),
            None => {
                at = html.len();
                break;
            }
        }
    }
    out.push_str(&html[at..]);
    out
}

/// The names under which the proxies of companies announce themselves, lower case, and how they
/// are written.
const PROXIES: &[(&str, &str)] = &[
    ("zscaler", "Zscaler"),
    ("netskope", "Netskope"),
    ("squid", "Squid"),
    ("bluecoat", "Blue Coat"),
    ("blue coat", "Blue Coat"),
    ("symantec", "Symantec"),
    ("forcepoint", "Forcepoint"),
    ("websense", "Websense"),
    ("fortigate", "FortiGate"),
    ("fortinet", "Fortinet"),
    ("palo alto", "Palo Alto"),
    ("umbrella", "Umbrella"),
    ("sophos", "Sophos"),
    ("skyhigh", "Skyhigh"),
    ("mcafee", "McAfee"),
    ("check point", "Check Point"),
    ("barracuda", "Barracuda"),
    ("iboss", "iboss"),
];

/// The proxy named in `text`, if one is.
fn known_proxy(text: &str) -> Option<&'static str> {
    let text = text.to_lowercase();
    PROXIES
        .iter()
        .find(|(name, _)| text.contains(name))
        .map(|(_, written)| *written)
}

/// What the answer says of itself when a proxy sent it: its `Proxy-Agent`, else its `Server` or
/// its `page` when they name a proxy known (GitHub's own `Server` is no proxy's), cut to a line.
/// Never its `Via`: every hop that relays an answer adds itself to it, so that a Squid says
/// « squid » even on a refusal of GitHub's, and the proxy would be blamed for it.
pub fn agent_of(headers: &HeaderMap, page: Option<&str>) -> Option<String> {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|v| !v.is_empty())
    };
    header("proxy-agent")
        .or_else(|| header("server").filter(|server| known_proxy(server).is_some()))
        .map(cut)
        .or_else(|| page.and_then(known_proxy).map(Into::into))
}

/// `host:port` of a proxy.
fn proxy_label(proxy: &Url) -> String {
    match (proxy.host_str(), proxy.port_or_known_default()) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        (Some(host), None) => host.into(),
        _ => proxy.to_string(),
    }
}

/// The client that asks as the plugin does: the same proxy (or none, or the system's when the
/// settings have none), TLS verification and name.
fn client(net: &Network, route: Route) -> reqwest::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .connect_timeout(PROBE_CONNECT)
        .user_agent(user_agent())
        .danger_accept_invalid_certs(net.insecure_tls);
    match net.via(route) {
        Via::Nothing => builder = builder.no_proxy(),
        Via::Proxy(proxy) => builder = builder.proxy(reqwest::Proxy::all(proxy.as_str())?),
        Via::System => {}
    }
    builder.build()
}

/// The text of an error page, if the answer is one: HTML or plain text, the first 8 KB read.
async fn read_page(response: &mut reqwest::Response) -> Option<String> {
    let kind = response
        .headers()
        .get(CONTENT_TYPE)?
        .to_str()
        .ok()?
        .to_ascii_lowercase();
    if !kind.starts_with("text/html") && !kind.starts_with("text/plain") {
        return None;
    }
    let mut bytes = Vec::new();
    while bytes.len() < PAGE_BYTES {
        match response.chunk().await {
            Ok(Some(chunk)) => bytes.extend_from_slice(&chunk),
            _ => break,
        }
    }
    bytes.truncate(PAGE_BYTES);
    // A character cut in two by the limit is not a word of the page.
    let text = page_text(String::from_utf8_lossy(&bytes).trim_end_matches('\u{fffd}'));
    (!text.is_empty()).then_some(text)
}

/// Asks `url` again, as the plugin did, to learn why it failed: `detail` is what the plugin said,
/// kept for what the probe cannot tell (an answer that is a success, but not a readable one).
/// A success is not read: an installer is 10 MB. The probe is a request of its own: a server
/// sees it after the plugin's.
pub async fn probe(
    url: &Url,
    net: &Network,
    route: Route,
    accept: &str,
    what: What,
    detail: String,
) -> Failure {
    let proxy = match net.via(route) {
        Via::Proxy(proxy) => Some(proxy_label(proxy)),
        _ => None,
    };
    let mut failure = Failure {
        what,
        url: url.to_string(),
        status: None,
        answered: false,
        host: url.host_str().unwrap_or_default().into(),
        proxy,
        agent: None,
        page: None,
        detail,
        stuck: None,
        trusted: Vec::new(),
        direct: None,
    };
    let sent = match client(net, route) {
        Ok(client) => client.get(url.clone()).header(ACCEPT, accept).send().await,
        Err(e) => Err(e),
    };
    let mut response = match sent {
        Ok(response) => response,
        Err(e) => {
            // The hop that could not be reached may be a redirection's.
            if let Some(host) = e.url().and_then(Url::host_str) {
                failure.host = host.into();
            }
            // The address is in the failure already, and a signed one is no one's to read.
            let e = e.without_url();
            // Through a proxy, which of its ways to fail this is (the other end is not the
            // proxy's). Without the address, which a word of it could mislead.
            if failure.proxy.is_some() {
                failure.stuck = stuck_of(&e, e.is_timeout());
            }
            failure.detail = chain(&e);
            return failure;
        }
    };
    failure.answered = true;
    failure.host = response.url().host_str().unwrap_or_default().into();
    let status = response.status();
    if !status.is_success() {
        failure.status = Some(status.as_u16());
        failure.page = read_page(&mut response).await;
    }
    failure.agent = agent_of(response.headers(), failure.page.as_deref());
    failure
}
