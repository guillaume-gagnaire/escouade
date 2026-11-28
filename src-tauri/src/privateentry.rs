//! The connectors' entry of `escouade-server` in the core (RP-4, `docs/PRIVATE-NETWORK.md` § 4.1, § 4.5, R3; spec
//! « Escouade à distance › Atteindre un réseau privé »). The entry itself is `escouade_gate::entry`; here, what the
//! core does with it:
//!
//! - **What is kept** ([`CoreStore`]): the list of the connectors and the trace of those removed in `connectors.json`
//!   (the data folder, readable by its owner alone, never a key), their keys, the server's and the pending codes' in
//!   `secrets.json` (`server::secrets`), each written by one process at a time (`escouade-server connector pair` and
//!   `connectors remove` beside the server).
//! - **Its start** ([`Core::start_private_network`]): with `--connector-listen` (the entry, and the gate with it) or
//!   `--gate` alone (the gate, no entry); the gate's connectors are the entry's ([`escouade_gate::entry::Registry`]).
//! - **The policy** `servers.privateNetwork` at `false` ([`Core::apply_private_network`]): at the start, neither the
//!   entry nor the gate opens; while the server runs, the entry closes and every channel is cut (the gate of the agents
//!   already started stays, reaching no private network: their way out to the Internet is not cut under them); the
//!   pairing is refused. Lifted, the entry opens again.

use crate::core::Core;
use crate::headless::with_file_lock;
use crate::paths::DataDir;
use crate::server::secrets::FileStore;
use anyhow::{Context, Result};
use escouade_gate::entry::{self, proto, Entry, Registry, Saved, Store};
use parking_lot::Mutex;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

/// The version the server judges a connector's by: its own.
pub fn server_version() -> proto::Version {
    proto::Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or(proto::Version::new(0, 0, 0))
}

/// The connectors kept by the core: `connectors.json` beside the data, the keys in the server's secrets.
pub struct CoreStore {
    file: PathBuf,
    secrets: Arc<FileStore>,
}

impl CoreStore {
    pub fn new(data: &DataDir, secrets: Arc<FileStore>) -> Self {
        Self {
            file: data.root().join(entry::store::FILE),
            secrets,
        }
    }

    fn read_file(&self) -> Result<Saved> {
        match std::fs::read(&self.file) {
            Ok(b) => serde_json::from_slice(&b).map_err(|e| {
                anyhow::anyhow!("{} unreadable (line {})", entry::store::FILE, e.line())
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Saved::default()),
            Err(e) => Err(anyhow::anyhow!("{}: {e}", entry::store::FILE)),
        }
    }
}

impl Store for CoreStore {
    fn read(&self) -> std::result::Result<Saved, String> {
        self.read_file().map_err(|e| format!("{e:#}"))
    }

    fn change(
        &self,
        change: &mut dyn FnMut(&mut Saved) -> bool,
    ) -> std::result::Result<(), String> {
        with_file_lock(&self.file, || {
            let mut saved = self.read_file()?;
            if !change(&mut saved) {
                return Ok(());
            }
            let bytes = serde_json::to_vec_pretty(&saved)?;
            crate::paths::write_private(&self.file, &bytes)
                .with_context(|| format!("writing {}", entry::store::FILE))
        })
        .map_err(|e| format!("{e:#}"))
    }

    fn secret(&self, name: &str) -> std::result::Result<Option<String>, String> {
        use crate::integrations::secrets::SecretStore;
        self.secrets.get(name).map_err(|e| format!("{e:#}"))
    }

    fn change_secret(
        &self,
        name: &str,
        change: &mut dyn FnMut(Option<String>) -> Option<String>,
    ) -> std::result::Result<(), String> {
        self.secrets
            .update(name, change)
            .map_err(|e| format!("{e:#}"))
    }
}

/// The connectors of the data folder `data`, for a command beside the server (`connector pair`, `connectors`).
pub fn registry_of(data: &DataDir, secrets: Arc<FileStore>) -> Arc<Registry> {
    Registry::new(
        Arc::new(CoreStore::new(data, secrets)),
        crate::privatenet::lang(),
        server_version(),
    )
}

/// What `escouade-server` was asked of the private network.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Asked {
    /// `--gate`: the gate, without the entry.
    pub gate: bool,
    /// `--connector-listen`: the entry (and the gate with it).
    pub listen: Option<SocketAddr>,
    /// `--connector-cert`, `--connector-key`: a certificate of its own (else behind a TLS proxy).
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    /// `--connector-url`: the entry's address as the connectors reach it (else made from the listening address).
    pub url: Option<String>,
    /// `--trust-proxy`: their `X-Forwarded-For` read.
    pub proxies: Vec<IpAddr>,
}

impl Asked {
    /// Whether anything of the private network was asked.
    pub fn any(&self) -> bool {
        self.gate || self.listen.is_some()
    }

    /// The entry's address as the connectors reach it: `--connector-url`, else `wss://<listening address or this
    /// machine's name>:<port>/connector`; and whether it was guessed.
    pub fn url_at(&self, addr: SocketAddr) -> (String, bool) {
        if let Some(u) = &self.url {
            return (u.trim().to_string(), false);
        }
        let host = if addr.ip().is_unspecified() {
            std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("COMPUTERNAME"))
                .ok()
                .map(|h| h.trim().to_lowercase())
                .filter(|h| !h.is_empty())
                .unwrap_or_else(|| "localhost".into())
        } else {
            match addr.ip() {
                IpAddr::V6(ip) => format!("[{ip}]"),
                ip => ip.to_string(),
            }
        };
        (format!("wss://{host}:{}{}", addr.port(), proto::PATH), true)
    }
}

/// The private network of the core: its connectors, and the entry while it is open.
#[derive(Default)]
pub struct EntrySlot {
    registry: Mutex<Option<Arc<Registry>>>,
    entry: Mutex<Option<Entry>>,
    asked: Mutex<Asked>,
}

impl EntrySlot {
    /// The connectors, on a server that serves the private network.
    pub fn registry(&self) -> Option<Arc<Registry>> {
        self.registry.lock().clone()
    }

    /// Whether the entry is open.
    pub fn is_open(&self) -> bool {
        self.entry.lock().is_some()
    }

    /// Where the entry listens, when it is open.
    pub fn addr(&self) -> Option<SocketAddr> {
        self.entry.lock().as_ref().map(Entry::addr)
    }

    /// What was asked.
    pub fn asked(&self) -> Asked {
        self.asked.lock().clone()
    }

    /// The entry closed, its channels cut.
    pub fn close(&self) {
        if let Some(e) = self.entry.lock().take() {
            e.stop();
        }
        if let Some(r) = self.registry() {
            if let Err(e) = r.set_entry_url(None) {
                log::warn!("connectors: {e}");
            }
        }
    }
}

fn tls_of(asked: &Asked) -> Result<Option<tokio_rustls::TlsAcceptor>> {
    match (&asked.cert, &asked.key) {
        (None, None) => Ok(None),
        (Some(cert), Some(key)) => {
            let chain = std::fs::read(cert).with_context(|| format!("{}", cert.display()))?;
            let key_pem = std::fs::read(key).with_context(|| format!("{}", key.display()))?;
            let acceptor = proto::transport::tls_acceptor(&chain, &key_pem)
                .map_err(|e| anyhow::anyhow!("the connectors' certificate: {e}"))?;
            Ok(Some(acceptor))
        }
        _ => anyhow::bail!(tr!(
            "--connector-cert et --connector-key vont ensemble",
            "--connector-cert and --connector-key go together"
        )),
    }
}

impl Core {
    /// The private network of this server (`escouade-server --connector-listen`, `--gate`): its connectors kept in
    /// `data` and `secrets`, the gate and the entry opened as `asked`, when the policies allow it. On the runtime.
    pub async fn start_private_network(
        self: &Arc<Self>,
        asked: Asked,
        secrets: Arc<FileStore>,
    ) -> Result<()> {
        let registry = registry_of(&self.data, secrets);
        registry.reset_connected();
        *self.private_network.registry.lock() = Some(registry);
        *self.private_network.asked.lock() = asked;
        self.apply_private_network().await
    }

    /// The private network as the policies now say: denied, the entry closed (and at the start, no gate); allowed,
    /// the gate and the entry opened as asked.
    pub async fn apply_private_network(self: &Arc<Self>) -> Result<()> {
        let asked = self.private_network.asked();
        let Some(registry) = self.private_network.registry() else {
            return Ok(());
        };
        if !asked.any() {
            return Ok(());
        }
        let denied = self.locks.read().check_private_network().err();
        if let Some(why) = denied {
            if self.private_network.is_open() {
                self.private_network.close();
                log::warn!(
                    "escouade-server: {why} The connectors' entry is closed, its channels cut."
                );
            } else if !self.gate.is_on() {
                log::warn!(
                    "escouade-server: {why} Neither the connectors' entry nor the gate is opened."
                );
            }
            return Ok(());
        }
        if !self.gate.is_on() {
            self.start_gate(registry.clone()).await?;
        }
        let Some(listen) = asked.listen else {
            return Ok(());
        };
        if self.private_network.is_open() {
            return Ok(());
        }
        let tls = tls_of(&asked)?;
        let has_tls = tls.is_some();
        let entry = Entry::start(entry::Config {
            listen,
            tls,
            registry: registry.clone(),
            proxies: asked.proxies.clone(),
            bounds: entry::Limits::default(),
            channel: proto::Limits::default(),
        })
        .await
        .with_context(|| format!("the connectors' entry on {listen}"))?;
        let (url, guessed) = asked.url_at(entry.addr());
        if let Err(e) = registry.set_entry_url(Some(url.clone())) {
            log::warn!("connectors: {e}");
        }
        log::info!(
            "escouade-server: the connectors' entry on {} ({url}{}{}): it speaks the connectors' protocol alone, never the remote API",
            entry.addr(),
            if guessed { ", guessed: --connector-url to say it" } else { "" },
            if has_tls { "" } else { ", without TLS: to serve behind a TLS proxy" }
        );
        *self.private_network.entry.lock() = Some(entry);
        Ok(())
    }

    /// A pairing code for a connector to be named `name` (the space's « Ajouter un connecteur »): refused when the
    /// policies deny the private network, or when this server serves none.
    pub fn pair_connector(&self, name: &str) -> Result<(entry::NewCode, Option<String>)> {
        self.locks.read().check_private_network()?;
        let registry = self.private_network.registry().ok_or_else(|| {
            anyhow::anyhow!(tr!(
                "ce serveur ne sert pas de réseau privé : lance escouade-server avec --connector-listen",
                "this server serves no private network: start escouade-server with --connector-listen"
            ))
        })?;
        let code = registry
            .create_code(name, crate::model::now_ms() as u64)
            .map_err(anyhow::Error::msg)?;
        Ok((code, registry.entry_url()))
    }
}

/// The private network as the space shows it (Réglages de l'espace › Réseau privé, RP-5).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Whether this server serves a private network (`--connector-listen` or `--gate`).
    pub served: bool,
    /// Why the policies deny it (`servers.privateNetwork`), when they do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied: Option<String>,
    /// The entry's address as the connectors reach it, while it is open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// Whether the gate runs.
    pub gate: bool,
    pub connectors: Vec<entry::View>,
    /// The codes that wait: the name to be and their end (never the code).
    pub pending: Vec<PendingCode>,
    pub removed: Vec<entry::Removed>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingCode {
    pub name: String,
    pub expires_at: u64,
}

/// A code made for the space: shown once, to the one who asked.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Code {
    pub code: String,
    pub expires_at: u64,
    /// The entry's address, to give the connector with the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
}

impl Core {
    /// The private network, as the space shows it.
    pub fn private_network_status(&self) -> Status {
        let denied = self
            .locks
            .read()
            .check_private_network()
            .err()
            .map(|e| e.to_string());
        let registry = self.private_network.registry();
        let now = crate::model::now_ms() as u64;
        Status {
            served: self.private_network.asked().any(),
            denied,
            entry: self
                .private_network
                .is_open()
                .then(|| registry.as_ref().and_then(|r| r.entry_url()))
                .flatten(),
            gate: self.gate.is_on(),
            connectors: registry.as_ref().map(|r| r.views()).unwrap_or_default(),
            pending: registry
                .as_ref()
                .map(|r| {
                    r.pending(now)
                        .into_iter()
                        .map(|p| PendingCode {
                            name: p.name,
                            expires_at: p.expires_at,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            removed: registry.as_ref().map(|r| r.removed()).unwrap_or_default(),
        }
    }

    /// The connector `id` removed from the space by `by`. False when there is none.
    pub fn remove_connector(&self, id: &str, by: &str) -> Result<bool> {
        let Some(registry) = self.private_network.registry() else {
            return Ok(false);
        };
        let removed = registry
            .remove(id, by, crate::model::now_ms() as u64)
            .map_err(anyhow::Error::msg)?;
        Ok(removed.is_some())
    }

    fn lists_file(&self) -> PathBuf {
        self.data.root().join(escouade_gate::lists::FILE)
    }

    /// The list of `project` (the hosts of the private networks it reaches).
    pub fn project_hosts(&self, project: &str) -> Result<escouade_gate::ProjectList> {
        let lists = escouade_gate::Lists::read(&self.lists_file())
            .with_context(|| escouade_gate::lists::FILE.to_string())?;
        Ok(lists.projects.get(project).cloned().unwrap_or_default())
    }

    /// The list of `project` written (empty: it reaches no private network): each host put in shape, of a connector
    /// paired, within that connector's ceiling when it is known (R4: the server cannot widen it). Never by an agent
    /// (R5): no MCP tool calls this. The gate reads it again at once.
    pub fn set_project_hosts(
        &self,
        project: &str,
        hosts: Vec<escouade_gate::Allowed>,
    ) -> Result<escouade_gate::ProjectList> {
        if !self.projects.read().iter().any(|p| p.id == project) {
            anyhow::bail!(tr!("aucun projet {project}", "no project {project}"));
        }
        let known = self
            .private_network
            .registry()
            .map(|r| escouade_gate::Connectors::known(r.as_ref()))
            .unwrap_or_default();
        let mut shaped = Vec::new();
        for a in hosts {
            let target = escouade_gate::Target::new(&a.host, a.port).ok_or_else(|| {
                anyhow::anyhow!(tr!(
                    "« {}:{} » n’est pas un hôte",
                    "“{}:{}” is not a host",
                    a.host,
                    a.port
                ))
            })?;
            let Some(c) = known.iter().find(|c| c.id == a.connector) else {
                anyhow::bail!(tr!(
                    "{target} : aucun connecteur {} n’est appairé",
                    "{target}: no connector {} is paired",
                    a.connector
                ));
            };
            if !c.ceiling.is_empty() && !c.ceiling.iter().any(|p| p.matches(&target)) {
                anyhow::bail!(tr!(
                    "{target} n’est pas dans le plafond du connecteur {} : seul le service informatique l’élargit, dans connector.toml",
                    "{target} is not in the ceiling of the connector {}: only the IT department widens it, in connector.toml",
                    c.name
                ));
            }
            if !shaped
                .iter()
                .any(|s: &escouade_gate::Allowed| s.host == target.host && s.port == target.port)
            {
                shaped.push(escouade_gate::Allowed {
                    host: target.host,
                    port: target.port,
                    connector: c.id.clone(),
                });
            }
        }
        let path = self.lists_file();
        let list = escouade_gate::ProjectList { hosts: shaped };
        with_file_lock(&path, || {
            let mut lists = escouade_gate::Lists::read(&path).unwrap_or_default();
            if list.hosts.is_empty() {
                lists.projects.remove(project);
            } else {
                lists.projects.insert(project.to_string(), list.clone());
            }
            let bytes = serde_json::to_vec_pretty(&lists)?;
            crate::paths::write_private(&path, &bytes)
                .with_context(|| format!("writing {}", escouade_gate::lists::FILE))
        })?;
        self.gate.reload_if_changed();
        log::info!(
            "private network: the list of the project {project} written ({} hosts)",
            list.hosts.len()
        );
        Ok(list)
    }

    /// The gate's journal: the last connections, the oldest first (none without the gate).
    pub fn gate_journal(&self) -> Vec<escouade_gate::Entry> {
        self.gate.gate().map(|g| g.journal()).unwrap_or_default()
    }
}
