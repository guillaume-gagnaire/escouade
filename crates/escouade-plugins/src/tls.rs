//! Whom the app's TLS connections trust (audit SEC-07): Mozilla's authorities, the system's, and
//! the file of Réglages › Avancé › Réseau's « Autorités de certification supplémentaires » (a
//! corporate proxy's root: Zscaler, a proxy that inspects TLS). The app's own requests, the
//! plugins' network (`fetch` and sockets) and the updates are made with the same configuration;
//! the agents get the file by their variables (`Settings::claude_env`).
//!
//! « Ignorer la vérification des certificats TLS », kept for an installation that has it, holds
//! for the connections that go through the proxy of the settings alone: with no proxy, or to a host
//! of its exclusions, a certificate is checked.

use anyhow::{bail, Context, Result};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::WebPkiServerVerifier;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

// The network settings and their proxy, which the settings of the agents read too: in `escouade-model` (docs/CRATES.md).
pub use escouade_model::tls::{proxy_url, Net};

/// The proxy `host` goes through: the settings', unless `no_proxy` names it (`excluded`). The one
/// reading of the exclusions (audit SEC-28): the route of every client of the settings
/// (`proxy`) and the check of a certificate (`Verifier`) ask it, so a certificate goes unchecked
/// only on a connection that goes through the proxy.
pub fn proxy_for(net: &Net, host: &str) -> Option<reqwest::Url> {
    if net.proxy.is_empty() || excluded(&net.no_proxy, host) {
        return None;
    }
    proxy_url(net)
}

/// The proxy of the settings as reqwest takes it, its route decided by `proxy_for` for each host;
/// None when the settings have no proxy, or one that is no address.
pub fn proxy(net: &Net) -> Option<reqwest::Proxy> {
    proxy_url(net)?;
    let net = net.clone();
    Some(reqwest::Proxy::custom(move |url| {
        url.host_str().and_then(|host| proxy_for(&net, host))
    }))
}

/// Whether the exclusions `no_proxy` send `host` around the proxy. Each, separated by a comma, a
/// space or a semicolon: `*` for all; an address, IPv6 in brackets or not; a range of addresses
/// (`10.0.0.0/8`, `fd00::/8`); a name, which holds for what is under it too, written with a
/// leading `.` or `*.`, a port or a final dot or not (the port is not compared).
pub fn excluded(no_proxy: &str, host: &str) -> bool {
    let host = host
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }
    let ip = host.parse::<std::net::IpAddr>().ok();
    no_proxy
        .split([',', ' ', ';', '\t', '\n', '\r'])
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .any(|e| exclusion_holds(&e.to_ascii_lowercase(), &host, ip))
}

fn exclusion_holds(entry: &str, host: &str, ip: Option<std::net::IpAddr>) -> bool {
    if entry == "*" {
        return true;
    }
    // An address or a range, IPv6 in brackets with its port after them.
    let addr = match entry.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or(""),
        None => entry,
    };
    if let Some((base, bits)) = addr.split_once('/') {
        return match (ip, base.parse::<std::net::IpAddr>(), bits.parse::<u32>()) {
            (Some(ip), Ok(base), Ok(bits)) => in_range(ip, base, bits),
            _ => false,
        };
    }
    if let Ok(a) = addr.parse::<std::net::IpAddr>() {
        return ip == Some(a);
    }
    // An address with its port (`10.1.2.3:8080`).
    if let Some((a, port)) = entry.rsplit_once(':') {
        if let (Ok(a), true) = (
            a.parse::<std::net::Ipv4Addr>(),
            port.bytes().all(|b| b.is_ascii_digit()),
        ) {
            return ip == Some(std::net::IpAddr::V4(a));
        }
    }
    if ip.is_some() {
        return false;
    }
    // A name: `.x`, `*.x`, `x`, its port and its final dot left aside.
    let name = entry.trim_start_matches('*').trim_start_matches('.');
    let name = match name.rsplit_once(':') {
        Some((n, port)) if port.bytes().all(|b| b.is_ascii_digit()) => n,
        _ => name,
    };
    let name = name.trim_end_matches('.');
    !name.is_empty() && (host == name || host.ends_with(&format!(".{name}")))
}

/// Whether `ip` is of the range `base/bits` (an IPv4 of an IPv6 range written `::ffff:…` too).
fn in_range(ip: std::net::IpAddr, base: std::net::IpAddr, bits: u32) -> bool {
    use std::net::IpAddr::{V4, V6};
    let ip = match ip {
        V6(v6) => v6.to_ipv4_mapped().map(V4).unwrap_or(ip),
        _ => ip,
    };
    match (ip, base) {
        (V4(ip), V4(base)) if bits <= 32 => {
            let mask = u32::MAX.checked_shl(32 - bits).unwrap_or(0);
            u32::from(ip) & mask == u32::from(base) & mask
        }
        (V6(ip), V6(base)) if bits <= 128 => {
            let mask = u128::MAX.checked_shl(128 - bits).unwrap_or(0);
            u128::from(ip) & mask == u128::from(base) & mask
        }
        _ => false,
    }
}

/// The certificates of a `.pem` file of authorities: an error the user can read when it holds
/// none, or is not read.
pub fn read_authorities(path: &str) -> Result<Vec<CertificateDer<'static>>> {
    let bytes = std::fs::read(path).with_context(|| {
        tr!(
            "fichier d'autorités illisible : {path}",
            "authorities file not readable: {path}"
        )
    })?;
    let certs: Vec<_> = CertificateDer::pem_slice_iter(&bytes)
        .filter_map(|c| c.ok())
        .collect();
    if certs.is_empty() {
        bail!(tr!(
            "aucun certificat PEM dans {path}",
            "no PEM certificate in {path}"
        ));
    }
    Ok(certs)
}

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Mozilla's authorities and the system's, read once.
fn base_roots() -> &'static [CertificateDer<'static>] {
    static ROOTS: OnceLock<Vec<CertificateDer<'static>>> = OnceLock::new();
    ROOTS.get_or_init(|| {
        let native = rustls_native_certs::load_native_certs();
        for e in &native.errors {
            log::warn!("tls: a root certificate of the system not read: {e}");
        }
        log::info!(
            "tls: {} root certificates of the system",
            native.certs.len()
        );
        native.certs
    })
}

/// The roots of `net`: Mozilla's, the system's and those of its file (one not read is said in
/// the log and left out: the others still hold).
fn roots(net: &Net) -> rustls::RootCertStore {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    roots.add_parsable_certificates(base_roots().iter().cloned());
    if !net.extra_ca.is_empty() {
        match read_authorities(&net.extra_ca) {
            Ok(certs) => {
                let (added, _) = roots.add_parsable_certificates(certs);
                log::info!("tls: {added} extra authorities from {}", net.extra_ca);
            }
            Err(e) => log::warn!("tls: {e:#}"),
        }
    }
    roots
}

/// The configuration of `net`'s connections, HTTP/1.1 (reqwest is built without HTTP/2, and a
/// WebSocket upgrades from it). Kept while the settings and the file are the same.
pub fn config(net: &Net) -> Arc<rustls::ClientConfig> {
    type Key = (Net, Option<SystemTime>);
    static CACHE: OnceLock<Mutex<HashMap<Key, Arc<rustls::ClientConfig>>>> = OnceLock::new();
    let stamp = (!net.extra_ca.is_empty())
        .then(|| {
            std::fs::metadata(&net.extra_ca)
                .and_then(|m| m.modified())
                .ok()
        })
        .flatten();
    let key = (net.clone(), stamp);
    let cache = CACHE.get_or_init(Default::default);
    if let Some(c) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return c.clone();
    }
    let built = build(net);
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() > 16 {
        cache.clear();
    }
    cache.insert(key, built.clone());
    built
}

fn build(net: &Net) -> Arc<rustls::ClientConfig> {
    let p = provider();
    let checked = WebPkiServerVerifier::builder_with_provider(Arc::new(roots(net)), p.clone())
        .build()
        .expect("roots of Mozilla at least");
    let verifier = Verifier {
        checked,
        unchecked_by: net.unchecked_by_proxy().then(|| net.clone()),
        provider: p.clone(),
    };
    let mut c = rustls::ClientConfig::builder_with_provider(p)
        .with_safe_default_protocol_versions()
        .expect("ring's protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    c.alpn_protocols = vec![b"http/1.1".to_vec()];
    Arc::new(c)
}

/// A reqwest client of `net`: its roots, and the certificates unchecked through the proxy alone
/// when the setting says so. The proxy itself is the caller's to set.
pub fn builder(net: &Net) -> reqwest::ClientBuilder {
    reqwest::Client::builder().use_preconfigured_tls((*config(net)).clone())
}

/// A client of the app's own requests: `builder` of the settings, through their proxy (its
/// exclusions around it, read by `proxy_for` as the certificates are).
pub fn app_builder(settings: &crate::model::Settings) -> reqwest::Result<reqwest::ClientBuilder> {
    let net = Net::of(settings);
    let mut b = builder(&net);
    if !net.proxy.is_empty() {
        // Checked as an address, as before: a proxy that is none is said, not gone around.
        reqwest::Proxy::all(net.proxy.as_str())?;
        b = match proxy(&net) {
            Some(proxy) => b.proxy(proxy),
            None => b.no_proxy(),
        };
    }
    Ok(b)
}

/// The certificates are checked against the roots, except, when « Ignorer la vérification » is on,
/// those of a host reached through the proxy.
#[derive(Debug)]
struct Verifier {
    checked: Arc<WebPkiServerVerifier>,
    unchecked_by: Option<Net>,
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl ServerCertVerifier for Verifier {
    fn verify_server_cert(
        &self,
        end: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if let Some(net) = &self.unchecked_by {
            let host = match name {
                ServerName::DnsName(d) => d.as_ref().to_string(),
                ServerName::IpAddress(ip) => std::net::IpAddr::from(*ip).to_string(),
                _ => String::new(),
            };
            if !host.is_empty() && proxy_for(net, &host).is_some() {
                return Ok(ServerCertVerified::assertion());
            }
        }
        self.checked
            .verify_server_cert(end, intermediates, name, ocsp, now)
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
