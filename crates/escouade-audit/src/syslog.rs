//! syslog (RFC 5424) over TCP or TLS (RFC 5425, octet counting; never UDP, which loses messages), and CEF in option
//! (`docs/TEAM-AUDIT.md` §4.2).
//!
//! A message: `<PRI>1 TIMESTAMP HOSTNAME escouade - MSGID [escouade@32473 …] MSG`, its `MSGID` the event, its
//! structured data a few parameters, its `MSG` the whole JSON record (or, with CEF, the record in CEF). Under 8 KiB:
//! the emitter refuses to seal an event that would not fit (`Emitter::seal`).

use std::io::{self, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::record::Record;
use crate::sink::Sink;

/// The IANA enterprise number of the structured data: 32473, the number RFC 5612 keeps for documentation, until the
/// user's PEN (`docs/TEAM-AUDIT.md` §8). The only place it is written.
pub const ENTERPRISE: u32 = 32473;
/// `APP-NAME`.
pub const APP_NAME: &str = "escouade";
/// The most bytes of a message (§4.2).
pub const MAX_MESSAGE: usize = 8 * 1024;
/// The facility: 13, log audit (RFC 5424 §6.2.1).
pub const FACILITY: u8 = 13;
/// The CEF header's vendor and product.
pub const CEF_VENDOR: &str = "Escouade";
pub const CEF_PRODUCT: &str = "escouade-team";

/// syslog's severity (0 emergency to 7 debug) of an event's severity (1 information to 5 alert): 5 → 2 critical,
/// 4 → 3 error, 3 → 4 warning, 2 → 5 notice, 1 → 6 informational. The event's own stays in the structured data.
pub fn syslog_severity(severity: u8) -> u8 {
    7 - severity.clamp(1, 5)
}

/// CEF's severity (0 to 10) of an event's: twice it.
pub fn cef_severity(severity: u8) -> u8 {
    severity.clamp(1, 5) * 2
}

/// How a message is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    /// `HOSTNAME` (printable ASCII, 255 at most; `-` when it is not).
    pub hostname: String,
    /// The record in CEF in `MSG`, rather than its JSON.
    pub cef: bool,
    /// The product's version, in CEF's header.
    pub version: String,
}

fn header_value(s: &str, max: usize) -> String {
    if s.is_empty() || s.len() > max || !s.bytes().all(|b| (33..=126).contains(&b)) {
        "-".to_owned()
    } else {
        s.to_owned()
    }
}

/// A parameter's value in structured data: `"`, `\` and `]` escaped (RFC 5424 §6.3.3).
fn sd_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '"' | '\\' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The message of `record`, without its frame.
pub fn message(record: &Record, f: &Format) -> String {
    let v = record.to_value();
    let pri = FACILITY * 8 + syslog_severity(record.severity);
    let sd = format!(
        "[escouade@{ENTERPRISE} schema=\"{}\" id=\"{}\" event=\"{}\" severity=\"{}\" team=\"{}\" source=\"{}\" device=\"{}\" seq=\"{}\"]",
        crate::SCHEMA,
        sd_value(record.id.as_str()),
        sd_value(record.fact.name()),
        record.severity,
        sd_value(record.team.id.as_str()),
        v["source"].as_str().unwrap_or("-"),
        sd_value(record.emitter.device.as_str()),
        record.emitter.seq,
    );
    let msg = if f.cef {
        cef(record, &v, &f.version)
    } else {
        serde_json::to_string(&v).expect("a record is JSON")
    };
    format!(
        "<{pri}>1 {} {} {APP_NAME} - {} {sd} {msg}",
        record.at,
        header_value(&f.hostname, 255),
        header_value(record.fact.name(), 32),
    )
}

/// The longest message `record` may make, whatever its host and format.
pub fn longest(record: &Record) -> usize {
    let host = "h".repeat(255);
    [false, true]
        .into_iter()
        .map(|cef| {
            message(
                record,
                &Format {
                    hostname: host.clone(),
                    cef,
                    version: "0.0.0-longest.version.here".to_owned(),
                },
            )
            .len()
        })
        .max()
        .unwrap_or(0)
}

/// The frame of RFC 5425 (octet counting): `LEN SP MSG`.
pub fn frame(message: &str) -> String {
    format!("{} {message}", message.len())
}

pub(crate) fn cef_header(s: &str) -> String {
    s.replace('\\', "\\\\").replace('|', "\\|")
}

pub(crate) fn cef_ext(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('=', "\\=")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

/// The record in CEF: its header, then its fields in CEF's keys (`rt`, `externalId`, `suid`, `suser`, `src`), and ours
/// in `cs1` to `cs6` (the team, the actor's device, the target and the detail in canonical JSON, the emitter's chain,
/// the anchor). Its signature does not go: `verify` reads the JSON lines or syslog's JSON.
pub fn cef(record: &Record, v: &Value, version: &str) -> String {
    let event = cef_header(record.fact.name());
    let mut ext: Vec<(String, String)> = vec![
        ("rt".into(), record.at.millis().to_string()),
        ("externalId".into(), record.id.as_str().to_owned()),
    ];
    let actor = record.actor();
    if let Some(m) = &actor.member {
        ext.push(("suid".into(), m.as_str().to_owned()));
    }
    if let Some(n) = &actor.name {
        ext.push(("suser".into(), n.as_str().to_owned()));
    }
    if let Some(ip) = &actor.ip {
        ext.push(("src".into(), ip.as_str().to_owned()));
    }
    let jcs = |p: &str| {
        String::from_utf8(crate::jcs::canonical(&v[p]).unwrap_or_default()).unwrap_or_default()
    };
    let anchor = record
        .anchor
        .as_ref()
        .map(|a| format!("{} {} {} {}", a.stream, a.n, a.epoch, a.ct))
        .unwrap_or_default();
    let emitter = format!(
        "{} {} {}",
        record.emitter.device,
        record.emitter.seq,
        record
            .emitter
            .prev
            .as_ref()
            .map(|d| d.as_str())
            .unwrap_or("-")
    );
    let custom = [
        ("team", record.team.id.as_str().to_owned()),
        (
            "device",
            actor
                .device
                .as_ref()
                .map(|d| d.as_str().to_owned())
                .unwrap_or_default(),
        ),
        ("target", jcs("target")),
        ("detail", jcs("detail")),
        ("emitter", emitter),
        ("anchor", anchor),
    ];
    for (i, (label, value)) in custom.into_iter().enumerate() {
        if !value.is_empty() {
            ext.push((format!("cs{}Label", i + 1), label.to_owned()));
            ext.push((format!("cs{}", i + 1), value));
        }
    }
    let ext = ext
        .into_iter()
        .map(|(k, v)| format!("{k}={}", cef_ext(&v)))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "CEF:0|{}|{}|{}|{event}|{event}|{}|{ext}",
        cef_header(CEF_VENDOR),
        cef_header(CEF_PRODUCT),
        cef_header(version),
        cef_severity(record.severity),
    )
}

/// Where syslog goes: `tls://host:port` (RFC 5425) or `tcp://host:port`. There is no UDP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub tls: bool,
    pub host: String,
    pub port: u16,
}

impl Target {
    pub fn parse(url: &str) -> Result<Target, String> {
        let (tls, rest) = if let Some(r) = url.strip_prefix("tls://") {
            (true, r)
        } else if let Some(r) = url.strip_prefix("tcp://") {
            (false, r)
        } else {
            return Err(format!(
                "« {url} »: tls://host:port or tcp://host:port (never UDP)"
            ));
        };
        let (host, port) = rest
            .rsplit_once(':')
            .ok_or_else(|| format!("« {url} »: no port"))?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let port: u16 = port.parse().map_err(|_| format!("« {url} »: no port"))?;
        if host.is_empty() {
            return Err(format!("« {url} »: no host"));
        }
        Ok(Target {
            tls,
            host: host.to_owned(),
            port,
        })
    }
}

/// The certificates syslog's TLS trusts: those of a PEM file (`--audit-syslog-ca`), else the system's.
pub fn roots(ca_pem: Option<&[u8]>) -> io::Result<rustls::RootCertStore> {
    use rustls_pki_types::pem::PemObject as _;
    let mut store = rustls::RootCertStore::empty();
    match ca_pem {
        Some(pem) => {
            for cert in rustls_pki_types::CertificateDer::pem_slice_iter(pem) {
                let cert =
                    cert.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                store
                    .add(cert)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            }
        }
        None => {
            for cert in rustls_native_certs::load_native_certs().certs {
                let _ = store.add(cert);
            }
        }
    }
    if store.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "no certificate to trust",
        ));
    }
    Ok(store)
}

type Stream = Box<dyn Write + Send>;

/// The syslog output: connects at its first message and again after a failure; a message it could not write is the
/// caller's to keep (the outbox keeps it on the disk).
pub struct Syslog {
    target: Target,
    format: Format,
    tls: Option<Arc<rustls::ClientConfig>>,
    timeout: Duration,
    stream: Option<Stream>,
}

impl Syslog {
    /// The output to `target`, trusting `roots` when it is TLS.
    pub fn new(
        target: Target,
        format: Format,
        roots: Option<rustls::RootCertStore>,
    ) -> io::Result<Syslog> {
        let tls = if target.tls {
            let roots = match roots {
                Some(r) => r,
                None => self::roots(None)?,
            };
            let config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .map_err(|e| io::Error::other(e.to_string()))?
            .with_root_certificates(roots)
            .with_no_client_auth();
            Some(Arc::new(config))
        } else {
            None
        };
        Ok(Syslog {
            target,
            format,
            tls,
            timeout: Duration::from_secs(10),
            stream: None,
        })
    }

    fn connect(&self) -> io::Result<Stream> {
        // Each address of the host in turn (`localhost` is `::1` before `127.0.0.1`).
        let mut last = io::Error::new(io::ErrorKind::NotFound, "no address");
        let mut connected = None;
        for addr in (self.target.host.as_str(), self.target.port).to_socket_addrs()? {
            match TcpStream::connect_timeout(&addr, self.timeout) {
                Ok(s) => {
                    connected = Some(s);
                    break;
                }
                Err(e) => last = e,
            }
        }
        let tcp = connected.ok_or(last)?;
        tcp.set_write_timeout(Some(self.timeout))?;
        tcp.set_read_timeout(Some(self.timeout))?;
        tcp.set_nodelay(true)?;
        match &self.tls {
            None => Ok(Box::new(tcp)),
            Some(config) => {
                let name = rustls_pki_types::ServerName::try_from(self.target.host.clone())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
                let conn = rustls::ClientConnection::new(config.clone(), name)
                    .map_err(|e| io::Error::other(e.to_string()))?;
                let mut tls = rustls::StreamOwned::new(conn, tcp);
                // The handshake now, for a refused certificate to fail here rather than at the first write's flush.
                while tls.conn.is_handshaking() {
                    tls.conn
                        .complete_io(&mut tls.sock)
                        .map_err(|e| io::Error::new(e.kind(), format!("TLS: {e}")))?;
                }
                Ok(Box::new(tls))
            }
        }
    }
}

impl Sink for Syslog {
    fn send(&mut self, line: &str) -> io::Result<()> {
        let record = Record::from_line(line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let msg = message(&record, &self.format);
        if msg.len() > MAX_MESSAGE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "over 8 KiB"));
        }
        if self.stream.is_none() {
            self.stream = Some(self.connect()?);
        }
        let stream = self.stream.as_mut().expect("connected");
        let sent = stream
            .write_all(frame(&msg).as_bytes())
            .and_then(|()| stream.flush());
        if sent.is_err() {
            self.stream = None;
        }
        sent
    }
}
