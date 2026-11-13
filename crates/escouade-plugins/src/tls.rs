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

/// The proxy `host` goes through: the settings', unless `no_proxy` names it (a host, a domain and
/// what is under it, `*` for all).
pub fn proxy_for(net: &Net, host: &str) -> Option<reqwest::Url> {
    if net.proxy.is_empty() {
        return None;
    }
    let host = host.trim_matches(['[', ']']).to_lowercase();
    let around = net
        .no_proxy
        .split([',', ' ', ';'])
        .map(|e| e.trim().trim_start_matches('.').to_lowercase())
        .filter(|e| !e.is_empty())
        .any(|e| e == "*" || host == e || host.ends_with(&format!(".{e}")));
    if around {
        return None;
    }
    proxy_url(net)
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
/// exclusions around it).
pub fn app_builder(settings: &crate::model::Settings) -> reqwest::Result<reqwest::ClientBuilder> {
    let mut b = builder(&Net::of(settings));
    let url = settings.proxy_url.trim();
    if !url.is_empty() {
        let proxy =
            reqwest::Proxy::all(url)?.no_proxy(reqwest::NoProxy::from_string(&settings.no_proxy));
        b = b.proxy(proxy);
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
