//! The phone's access to the core (ESC-567, MO-2 of `docs/MOBILE.md`; `docs/REMOTE.md`, « Le téléphone »): a
//! WireGuard peer in user space on a UDP port of the machine (`stack`), the remote API served inside the tunnel alone
//! (on the desktop, the API has no socket of its own then: `Listen::Nothing`), and what it takes to reach it from
//! outside (`nat`: the port opened in the box by UPnP or NAT-PMP, a CGNAT, the public address read again), or
//! Tailscale when it is there (`tailscale`: the API served on the machine's address in the tailnet).
//!
//! - **Off by default**, and never while a policy of the device forbids it (`servers.remoteApi`, `servers.phone`):
//!   checked when it starts, and every two seconds while it runs (`watch`), and when the policies change.
//! - **What is kept** (`phone.json`, beside the data): on or off, the port, Tailscale on or off (off until the user
//!   turns it on), STUN on or off, the peers (a phone's
//!   name, public key, address in the tunnel, the device it was paired as), the public address seen and when it
//!   changed. The tunnel's private key is in the keychain (`phone-tunnel-key`), or in that file, readable by its owner
//!   alone, when the keychain refuses it (Linux, a server).
//! - **A peer** belongs to the device paired through it (`TunnelPeer::paired`): only that device's token is taken
//!   through it; a device revoked takes its peer with it.
//! - **The phone's side** (its app, its key, the QR code) is MO-3 and MO-4: here, the tests' peer is a WireGuard of
//!   Rust (`stack`) that connects to the core like the phone will.

pub mod nat;
pub mod stack;
pub mod tailscale;

use crate::core::Core;
use crate::integrations::secrets::{keep_secret, SecretStore};
use crate::model::{new_id, now_ms};
use crate::paths::DataDir;
use crate::remoteapi::server::{RemoteApi, TunnelPeer, Via};
use anyhow::{anyhow, bail, Context, Result};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Weak};
use std::time::Duration;

/// The file of the phone's access, in the data folder.
pub const FILE: &str = "phone.json";
/// The UDP port proposed (WireGuard's usual one).
pub const DEFAULT_PORT: u16 = 51820;
/// The core's address in the tunnel.
pub const TUNNEL_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 77, 0, 1);
/// The tunnel's network.
pub const TUNNEL_PREFIX: u8 = 24;
/// The TCP port the API is served on in the tunnel.
pub const API_PORT: u16 = 80;
/// The phones at most (the addresses of the tunnel's network).
pub const MAX_PEERS: usize = 253;
/// The keychain's entry of the tunnel's private key.
const KEY_ENTRY: &str = "phone-tunnel-key";
/// How often the network is looked at again (the public address, the box's mapping renewed).
const CHECK_EVERY: Duration = Duration::from_secs(5 * 60);
/// How often the policies and the devices are looked at while it runs.
const WATCH_EVERY: Duration = Duration::from_secs(2);
/// The variable that turns the network's probes off in a debug build (`off`: the e2e tests, nothing leaves the
/// machine). Read in a debug build alone.
pub const NET_VAR: &str = "ESCOUADE_PHONE_NET";

/// A phone the tunnel knows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub id: String,
    pub name: String,
    /// Its WireGuard public key (base64).
    pub public_key: String,
    /// Its address in the tunnel: the only one its packets may come from.
    pub address: Ipv4Addr,
    /// The device of the remote API it was paired as: only that token is taken through it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    pub added_at: i64,
}

/// The public address, as last seen, and when it changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seen {
    pub address: String,
    pub since: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_at: Option<i64>,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn yes() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Saved {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_port")]
    port: u16,
    /// The API served on Tailscale's address: off until the user turns it on, once Réglages › Téléphone shows
    /// Tailscale found (SR1-05).
    #[serde(default)]
    tailscale: bool,
    /// The public address asked of the STUN servers (Cloudflare's, Google's) at each look at the network; off, the
    /// box (UPnP, NAT-PMP) and the address given by hand still serve (SR1-11).
    #[serde(default = "yes")]
    stun: bool,
    #[serde(default)]
    peers: Vec<Peer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    public: Option<Seen>,
    /// The tunnel's public key (base64), to show it without reading the private one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    public_key: Option<String>,
    /// The private key, here only when the keychain refused it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key: Option<String>,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            tailscale: false,
            stun: true,
            peers: Vec::new(),
            public: None,
            public_key: None,
            key: None,
        }
    }
}

/// The state of the access, as Réglages › Téléphone shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Off,
    Starting,
    On,
    Error,
}

/// A phone, as Réglages › Téléphone shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerStatus {
    pub id: String,
    pub name: String,
    pub address: String,
    pub device: Option<String>,
    pub added_at: i64,
    /// Milliseconds since its last handshake; none when its session is down.
    pub since_handshake: Option<u64>,
}

/// What Réglages › Téléphone shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub state: State,
    /// Why it is off or failed: a policy, the port taken.
    pub error: Option<String>,
    /// The policy that forbids it, said (« par la stratégie de ce poste »).
    pub denied: Option<String>,
    pub port: u16,
    pub default_port: u16,
    /// Where it listens (`[::]:51820`) once on.
    pub bound: Option<String>,
    pub tunnel_address: String,
    /// The API's address in the tunnel.
    pub api: String,
    pub public_key: Option<String>,
    /// The core of a server (`escouade-server`): its firewall, no box.
    pub server: bool,
    pub net: Option<nat::NetState>,
    pub checking: bool,
    pub checked_at: Option<i64>,
    pub public: Option<Seen>,
    pub tailscale: Option<tailscale::Found>,
    pub tailscale_on: bool,
    /// Where the API is served on Tailscale's address, once it is.
    pub tailscale_served: Option<String>,
    /// Whether the public address is asked of the STUN servers.
    pub stun_on: bool,
    /// The STUN servers asked when it is on (`host:port`), as Réglages › Téléphone says them.
    pub stun_servers: Vec<String>,
    pub peers: Vec<PeerStatus>,
    pub handshakes: u64,
    pub unknown: u64,
}

/// The API the tunnel serves, made when the access goes on (the desktop: an API without a socket of its own).
pub type Factory =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<Arc<RemoteApi>>> + Send>> + Send + Sync>;

struct Running {
    tunnel: Arc<stack::Tunnel>,
    port: u16,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    api: Arc<RemoteApi>,
    /// The API made for the tunnel (the desktop's), stopped with it.
    own_api: bool,
    tailscale: Option<(SocketAddr, tokio::task::JoinHandle<()>)>,
}

#[derive(Default)]
struct Live {
    error: Option<String>,
    starting: bool,
    net: Option<nat::NetState>,
    checking: bool,
    checked_at: Option<i64>,
    tailscale: Option<tailscale::Found>,
}

/// The phone's access of a core.
pub struct Phone {
    me: Weak<Phone>,
    file: PathBuf,
    devices_file: PathBuf,
    secrets: Arc<dyn SecretStore>,
    saved: Mutex<Saved>,
    run: Mutex<Option<Running>>,
    live: Mutex<Live>,
    probes: RwLock<nat::Probes>,
    tailscale_probe: RwLock<tailscale::Probe>,
    /// The API of the process (a server's, a test's), when it has one already.
    api: Mutex<Option<Weak<RemoteApi>>>,
    factory: Mutex<Option<Factory>>,
    server: std::sync::atomic::AtomicBool,
    /// One change of the access at a time.
    applying: tokio::sync::Mutex<()>,
    /// One change of Tailscale's socket at a time (two looks at the network may end together).
    tailscale_lock: tokio::sync::Mutex<()>,
    /// The file as it was last read or written (its time and length): `escouade-server phone` changes it from another
    /// process, and `reload_if_changed` reads it again.
    stamp: Mutex<Option<(std::time::SystemTime, u64)>>,
}

/// What `escouade-server phone` says of the access.
pub struct Summary {
    pub enabled: bool,
    pub port: u16,
    pub public_key: Option<String>,
}

fn stamp_of(file: &std::path::Path) -> Option<(std::time::SystemTime, u64)> {
    let meta = std::fs::metadata(file).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// What `ESCOUADE_PHONE_NET` asks, in a debug build alone: `off` (nothing looked at), or the e2e tests' fake box and
/// STUN server on the loopback (`fake;natpmp=127.0.0.1:5351;stun=127.0.0.1:3478;local=192.168.1.20`). None: the real
/// network.
fn probes_asked() -> Option<nat::Probes> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let asked = std::env::var(NET_VAR).ok()?;
    if asked == "off" {
        return Some(nat::Probes::off());
    }
    let rest = asked.strip_prefix("fake;")?;
    let mut probes = nat::Probes::off();
    probes.wait = Duration::from_secs(1);
    for part in rest.split(';') {
        match part.split_once('=') {
            Some(("natpmp", at)) => {
                probes.natpmp = at.parse().ok();
                probes.natpmp_on = probes.natpmp.is_some();
            }
            Some(("stun", at)) => probes.stun = vec![at.to_string()],
            Some(("local", ip)) => probes.local = ip.parse().ok(),
            _ => {}
        }
    }
    Some(probes)
}

fn probes_of_process() -> nat::Probes {
    probes_asked().unwrap_or_else(nat::Probes::real)
}

impl Phone {
    /// The access of the core of `data`, as its file keeps it; nothing runs until `apply`.
    pub fn load(data: &DataDir, secrets: Arc<dyn SecretStore>) -> Arc<Self> {
        let file = data.root().join(FILE);
        let saved = match std::fs::read(&file) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                log::error!("phone: {FILE} unreadable ({e}): the access starts off");
                Saved::default()
            }),
            Err(_) => Saved::default(),
        };
        // The tests' network: no Tailscale of the machine either.
        let net_off = probes_asked().is_some();
        let stamp = stamp_of(&file);
        Arc::new_cyclic(|me| Self {
            me: me.clone(),
            file,
            devices_file: data.root().join(crate::remoteapi::devices::FILE),
            secrets,
            stamp: Mutex::new(stamp),
            saved: Mutex::new(saved),
            run: Mutex::new(None),
            live: Mutex::new(Live::default()),
            probes: RwLock::new(probes_of_process()),
            tailscale_probe: RwLock::new(if net_off {
                tailscale::Probe::Given(None)
            } else {
                tailscale::Probe::Real
            }),
            api: Mutex::new(None),
            factory: Mutex::new(None),
            server: std::sync::atomic::AtomicBool::new(false),
            applying: tokio::sync::Mutex::new(()),
            tailscale_lock: tokio::sync::Mutex::new(()),
        })
    }

    /// Where the network's probes look (the tests: fakes on the loopback).
    pub fn set_probes(&self, probes: nat::Probes) {
        *self.probes.write() = probes;
    }

    /// How Tailscale is looked for (the tests: as given).
    pub fn set_tailscale_probe(&self, probe: tailscale::Probe) {
        *self.tailscale_probe.write() = probe;
    }

    /// The process's API, the tunnel's from now on (`escouade-server`, a test).
    pub fn attach(&self, api: &Arc<RemoteApi>) {
        *self.api.lock() = Some(Arc::downgrade(api));
    }

    /// How the desktop makes the API the tunnel serves, when it has none.
    pub fn set_factory(&self, factory: Factory) {
        *self.factory.lock() = Some(factory);
    }

    /// The core of a server (`escouade-server`): its firewall to open, no box.
    pub fn mark_server(&self) {
        self.server.store(true, Ordering::Relaxed);
    }

    fn save(&self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&*self.saved.lock())?;
        crate::paths::write_private(&self.file, &bytes)
            .with_context(|| format!("writing {FILE}"))?;
        *self.stamp.lock() = stamp_of(&self.file);
        Ok(())
    }

    /// The file read again when another process changed it (`escouade-server phone`): true when it did, for the
    /// caller to apply it. The running tunnel takes its peers at once.
    pub fn reload_if_changed(&self) -> bool {
        let now = stamp_of(&self.file);
        if *self.stamp.lock() == now {
            return false;
        }
        *self.stamp.lock() = now;
        let Ok(bytes) = std::fs::read(&self.file) else {
            return false;
        };
        match serde_json::from_slice::<Saved>(&bytes) {
            Ok(saved) => {
                *self.saved.lock() = saved;
                self.push_peers();
                log::info!("phone: {FILE} changed by another process: read again");
                true
            }
            Err(e) => {
                log::warn!("phone: {FILE} unreadable ({e}): kept as it was");
                false
            }
        }
    }

    /// On or off, the port, as kept (`escouade-server phone on|off`): applied by the process that runs the core.
    pub fn summary(&self) -> Summary {
        let saved = self.saved.lock();
        Summary {
            enabled: saved.enabled,
            port: saved.port,
            public_key: saved.public_key.clone(),
        }
    }

    /// The access on or off, its port, written for the process that runs the core (`escouade-server phone on|off`).
    pub fn set_saved(&self, enabled: Option<bool>, port: Option<u16>) -> Result<()> {
        if port == Some(0) {
            bail!(tr!("un port de 1 à 65535", "a port from 1 to 65535"));
        }
        {
            let mut saved = self.saved.lock();
            if let Some(e) = enabled {
                saved.enabled = e;
            }
            if let Some(p) = port {
                saved.port = p;
            }
        }
        // The key made the first time, or read (never made again over one the keychain holds).
        self.private_key()?;
        self.save()
    }

    /// A phone added with a key made here (`escouade-server phone add`, the official WireGuard app until MO-4's
    /// pairing): its WireGuard config, which holds its private key (given once, kept nowhere), and the peer.
    /// `endpoint`: where it reaches the server (else the public address last seen).
    pub fn add_phone(&self, name: &str, endpoint: Option<&str>) -> Result<(String, Peer)> {
        // The key made the first time, or read (never made again over one the keychain holds).
        self.private_key()?;
        let (private, public) = stack::new_key();
        let peer = self.add_peer(name, &stack::key_text(&public))?;
        let saved = self.saved.lock().clone();
        let host = endpoint
            .map(str::to_string)
            .or_else(|| saved.public.as_ref().map(|p| p.address.clone()))
            .unwrap_or_else(|| tr!("ADRESSE-DU-SERVEUR", "SERVER-ADDRESS"));
        let host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]")
        } else {
            host
        };
        let config = format!(
            "[Interface]\nPrivateKey = {}\nAddress = {}/32\n\n[Peer]\nPublicKey = {}\nAllowedIPs = {TUNNEL_ADDRESS}/32\nEndpoint = {host}:{}\nPersistentKeepalive = 25\n",
            stack::key_text(&private.to_bytes()),
            peer.address,
            saved.public_key.unwrap_or_default(),
            saved.port
        );
        Ok((config, peer))
    }

    // ---------- the key ----------

    /// The tunnel's private key: read, or made the first time (the keychain, or the file).
    fn private_key(&self) -> Result<boringtun::x25519::StaticSecret> {
        let in_file = self.saved.lock().key.clone();
        let text = match in_file {
            Some(k) => Some(k),
            None => self.secrets.get(KEY_ENTRY).ok().flatten(),
        };
        if let Some(bytes) = text.as_deref().and_then(stack::key_of) {
            let private = boringtun::x25519::StaticSecret::from(bytes);
            // Its public key said in the file, from the key itself (a file written before it, or lost).
            let public = stack::key_text(&boringtun::x25519::PublicKey::from(&private).to_bytes());
            let missing = self.saved.lock().public_key.as_deref() != Some(public.as_str());
            if missing {
                self.saved.lock().public_key = Some(public);
                self.save()?;
            }
            return Ok(private);
        }
        self.new_key().map(|(private, _)| private)
    }

    /// A new key pair for the tunnel, kept: its private key, and its public key (base64).
    fn new_key(&self) -> Result<(boringtun::x25519::StaticSecret, String)> {
        let (private, public) = stack::new_key();
        let text = stack::key_text(&private.to_bytes());
        let in_file = match keep_secret(&*self.secrets, KEY_ENTRY, &text) {
            Ok(()) => None,
            Err(e) => {
                log::warn!("phone: the keychain refused the tunnel's key ({e:#}): kept in {FILE}");
                Some(text)
            }
        };
        let public = stack::key_text(&public);
        {
            let mut saved = self.saved.lock();
            saved.key = in_file;
            saved.public_key = Some(public.clone());
        }
        self.save()?;
        Ok((private, public))
    }

    /// The tunnel's key changed (Réglages › Téléphone › Avancé): every phone must take the new public key (its QR code
    /// made again), the sessions of the old one are dropped and no longer answered. The new public key.
    pub fn rotate_key(&self) -> Result<String> {
        let (private, public) = self.new_key()?;
        if let Some(run) = self.run.lock().as_ref() {
            run.tunnel.set_key(private);
        }
        log::info!("phone: the tunnel's key changed");
        Ok(public)
    }

    // ---------- the peers ----------

    fn peer_configs(&self) -> Vec<stack::PeerConfig> {
        self.saved
            .lock()
            .peers
            .iter()
            .filter_map(|p| {
                Some(stack::PeerConfig {
                    id: p.id.clone(),
                    public: stack::key_of(&p.public_key)?,
                    address: p.address,
                    endpoint: None,
                    keepalive: None,
                })
            })
            .collect()
    }

    fn push_peers(&self) {
        let configs = self.peer_configs();
        if let Some(run) = self.run.lock().as_ref() {
            run.tunnel.set_peers(configs);
        }
    }

    /// A phone added by its public key (base64), named `name`: its address in the tunnel, the next free one. The
    /// QR code (MO-4) gives it its config; until then, the tests add theirs.
    pub fn add_peer(&self, name: &str, public_key: &str) -> Result<Peer> {
        let Some(key) = stack::key_of(public_key) else {
            bail!(tr!("clé publique illisible", "unreadable public key"));
        };
        let peer = {
            let mut saved = self.saved.lock();
            if saved
                .peers
                .iter()
                .any(|p| stack::key_of(&p.public_key) == Some(key))
            {
                bail!(tr!(
                    "ce téléphone est déjà là",
                    "this phone is there already"
                ));
            }
            if saved.peers.len() >= MAX_PEERS {
                bail!(tr!(
                    "{MAX_PEERS} téléphones au plus",
                    "{MAX_PEERS} phones at most"
                ));
            }
            let taken: Vec<Ipv4Addr> = saved.peers.iter().map(|p| p.address).collect();
            let address = (2..=254u8)
                .map(|n| Ipv4Addr::new(10, 77, 0, n))
                .find(|a| !taken.contains(a))
                .ok_or_else(|| anyhow!("no address left in the tunnel"))?;
            let name: String = name.chars().filter(|c| !c.is_control()).take(64).collect();
            let peer = Peer {
                id: new_id(),
                name: if name.trim().is_empty() {
                    tr!("Téléphone", "Phone")
                } else {
                    name
                },
                public_key: stack::key_text(&key),
                address,
                device: None,
                added_at: now_ms(),
            };
            saved.peers.push(peer.clone());
            peer
        };
        self.save()?;
        self.push_peers();
        log::info!("phone: peer « {} » added at {}", peer.name, peer.address);
        Ok(peer)
    }

    /// The phone `id` removed: its sessions and connections cut. False when there was none.
    /// Its device's token stays: `take_peer` says which it was, for Réglages › Téléphone to revoke it (SR1-03).
    pub fn remove_peer(&self, id: &str) -> Result<bool> {
        Ok(self.take_peer(id)?.is_some())
    }

    /// The phone `id` removed, as `remove_peer`: the peer it was (its `device`, the device it was paired as), none when
    /// there was none.
    pub fn take_peer(&self, id: &str) -> Result<Option<Peer>> {
        let gone = {
            let mut saved = self.saved.lock();
            let gone = saved.peers.iter().find(|p| p.id == id).cloned();
            saved.peers.retain(|p| p.id != id);
            gone
        };
        if gone.is_some() {
            self.save()?;
            self.push_peers();
            log::info!("phone: peer {id} removed");
        }
        Ok(gone)
    }

    pub fn peers(&self) -> Vec<Peer> {
        self.saved.lock().peers.clone()
    }

    /// The peer `id` belongs to `device` from now on (paired through it).
    fn link(&self, id: &str, device: &str) {
        let changed = {
            let mut saved = self.saved.lock();
            match saved.peers.iter_mut().find(|p| p.id == id) {
                Some(p) if p.device.as_deref() != Some(device) => {
                    p.device = Some(device.to_string());
                    true
                }
                _ => false,
            }
        };
        if changed {
            if let Err(e) = self.save() {
                log::error!("phone: a peer's device not kept: {e:#}");
            }
        }
    }

    /// The peers whose device was revoked, removed with it.
    fn drop_revoked(&self) {
        let linked: Vec<(String, String)> = self
            .saved
            .lock()
            .peers
            .iter()
            .filter_map(|p| Some((p.id.clone(), p.device.clone()?)))
            .collect();
        if linked.is_empty() {
            return;
        }
        let held: Vec<String> = std::fs::read(&self.devices_file)
            .ok()
            .and_then(|b| serde_json::from_slice::<Vec<serde_json::Value>>(&b).ok())
            .unwrap_or_default()
            .iter()
            .filter_map(|d| d["id"].as_str().map(str::to_string))
            .collect();
        for (peer, device) in linked {
            if !held.contains(&device) {
                log::info!("phone: the device {device} was revoked: its peer goes with it");
                let _ = self.remove_peer(&peer);
            }
        }
    }

    // ---------- on, off ----------

    /// The access changed (Réglages › Téléphone): on or off, its port, Tailscale on or off; then applied.
    pub async fn set(
        &self,
        core: &Arc<Core>,
        enabled: Option<bool>,
        port: Option<u16>,
        tailscale: Option<bool>,
    ) -> Result<()> {
        if enabled == Some(true) {
            core.locks.read().check_phone()?;
        }
        if port == Some(0) {
            bail!(tr!("un port de 1 à 65535", "a port from 1 to 65535"));
        }
        {
            let mut saved = self.saved.lock();
            if let Some(e) = enabled {
                saved.enabled = e;
            }
            if let Some(p) = port {
                saved.port = p;
            }
            if let Some(t) = tailscale {
                saved.tailscale = t;
            }
        }
        self.save()?;
        self.apply(core).await
    }

    /// The public address asked of the STUN servers or not (Réglages › Téléphone), from the next look at the network.
    pub fn set_stun(&self, on: bool) -> Result<()> {
        self.saved.lock().stun = on;
        self.save()
    }

    /// What the file and the policies say, made true: started, restarted on another port, or stopped.
    pub async fn apply(&self, core: &Arc<Core>) -> Result<()> {
        let _one = self.applying.lock().await;
        let (enabled, port) = {
            let saved = self.saved.lock();
            (saved.enabled, saved.port)
        };
        let allowed = core.locks.read().check_phone();
        if !enabled || allowed.is_err() {
            self.stop_running().await;
            let mut live = self.live.lock();
            live.error = if enabled {
                allowed.err().map(|e| e.to_string())
            } else {
                None
            };
            live.starting = false;
            return Ok(());
        }
        let running = self.run.lock().as_ref().map(|r| r.port);
        if running == Some(port) {
            self.apply_tailscale().await;
            return Ok(());
        }
        if running.is_some() {
            self.stop_running().await;
        }
        self.live.lock().starting = true;
        let started = self.start(core, port).await;
        let mut live = self.live.lock();
        live.starting = false;
        match started {
            Ok(()) => {
                live.error = None;
                Ok(())
            }
            Err(e) => {
                let text = e.to_string();
                log::warn!("phone: the access did not start: {e:#}");
                live.error = Some(text);
                Err(e)
            }
        }
    }

    async fn api_for(&self) -> Result<(Arc<RemoteApi>, bool)> {
        if let Some(api) = self.api.lock().as_ref().and_then(Weak::upgrade) {
            return Ok((api, false));
        }
        let made = {
            let factory = self.factory.lock();
            match factory.as_ref() {
                Some(f) => f(),
                None => bail!("no remote API in this process"),
            }
        };
        Ok((made.await?, true))
    }

    async fn start(&self, core: &Arc<Core>, port: u16) -> Result<()> {
        let private = self.private_key()?;
        let socket = stack::bind_udp(port).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                anyhow!(tr!(
                    "Le port UDP {port} est déjà pris sur cette machine (un autre WireGuard ?) : choisis-en un autre.",
                    "UDP port {port} is already taken on this machine (another WireGuard?): choose another one."
                ))
            } else {
                anyhow!(tr!(
                    "Le port UDP {port} ne s'ouvre pas : {e}",
                    "UDP port {port} does not open: {e}"
                ))
            }
        })?;
        let (api, own_api) = self.api_for().await?;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<stack::Accepted>(16);
        let tunnel = Arc::new(stack::Tunnel::start(
            socket,
            stack::Config {
                private,
                address: TUNNEL_ADDRESS,
                prefix: TUNNEL_PREFIX,
                listen: Some(API_PORT),
                peers: self.peer_configs(),
            },
            Some(tx),
        )?);
        let bound = tunnel.local_addr();
        log::info!("phone: the tunnel listens on UDP {bound}, the API in it on {TUNNEL_ADDRESS}:{API_PORT}");
        let mut tasks = Vec::new();
        // The tunnel's connections, to the API.
        let me = self.me.clone();
        let served = api.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(accepted) = rx.recv().await {
                let link = Arc::new(PeerLink {
                    phone: me.clone(),
                    id: accepted.peer,
                });
                served.serve_stream(Box::new(accepted.stream), Via::Tunnel(link));
            }
        }));
        // The policies and the devices, while it runs; the network, every few minutes.
        let me = self.me.clone();
        let weak_core = Arc::downgrade(core);
        tasks.push(tokio::spawn(async move {
            let mut checked = tokio::time::Instant::now();
            loop {
                tokio::time::sleep(WATCH_EVERY).await;
                let (Some(phone), Some(core)) = (me.upgrade(), weak_core.upgrade()) else {
                    return;
                };
                if let Err(e) = core.locks.read().check_phone() {
                    // A policy came: off at once.
                    phone.live.lock().error = Some(e.to_string());
                    phone.shutdown();
                    return;
                }
                phone.drop_revoked();
                if checked.elapsed() >= CHECK_EVERY {
                    checked = tokio::time::Instant::now();
                    phone.check().await;
                }
            }
        }));
        *self.run.lock() = Some(Running {
            tunnel,
            port,
            tasks,
            api,
            own_api,
            tailscale: None,
        });
        // The first look at the network, now that it runs: the box asked to open the port.
        let me = self.me.clone();
        let first = tokio::spawn(async move {
            if let Some(phone) = me.upgrade() {
                phone.check().await;
            }
        });
        if let Some(run) = self.run.lock().as_mut() {
            run.tasks.push(first);
        }
        Ok(())
    }

    /// Stopped: the tunnel, its tasks, the API made for it, Tailscale's socket; the mapping removed from the box.
    async fn stop_running(&self) {
        let Some(run) = self.run.lock().take() else {
            return;
        };
        Self::halt(&run);
        let opened = self.live.lock().net.as_ref().and_then(|n| n.opened.clone());
        if let Some(opened) = opened {
            let probes = self.probes.read().clone();
            nat::close(&opened, run.port, &probes).await;
        }
        let mut live = self.live.lock();
        if let Some(net) = live.net.as_mut() {
            net.opened = None;
        }
        log::info!("phone: the access is off");
    }

    fn halt(run: &Running) {
        run.tunnel.stop();
        for task in &run.tasks {
            task.abort();
        }
        if let Some((_, task)) = &run.tailscale {
            task.abort();
        }
        if run.own_api {
            run.api.stop();
        }
    }

    /// Stopped at once, as when the app quits or a policy comes: the box's mapping removed in the background (it
    /// expires on its own when the app is gone first).
    pub fn shutdown(&self) {
        let Some(run) = self.run.lock().take() else {
            return;
        };
        Self::halt(&run);
        let opened = self.live.lock().net.as_mut().and_then(|n| n.opened.take());
        if let Some(opened) = opened {
            let probes = self.probes.read().clone();
            let port = run.port;
            tauri::async_runtime::spawn(async move { nat::close(&opened, port, &probes).await });
        }
        log::info!("phone: the access is off");
    }

    /// Whether it is wanted on and has an API to serve: what a lifted policy turns on again.
    pub fn ready(&self) -> bool {
        self.saved.lock().enabled
            && (self.factory.lock().is_some()
                || self
                    .api
                    .lock()
                    .as_ref()
                    .is_some_and(|a| a.strong_count() > 0))
    }

    /// When the network was last looked at.
    pub fn checked_at(&self) -> Option<i64> {
        self.live.lock().checked_at
    }

    pub fn running(&self) -> bool {
        self.run.lock().is_some()
    }

    /// Where its UDP socket is bound, once on.
    pub fn bound(&self) -> Option<SocketAddr> {
        self.run.lock().as_ref().map(|r| r.tunnel.local_addr())
    }

    // ---------- the network ----------

    /// The network looked at now: the box, the public address, Tailscale.
    pub async fn check(&self) {
        let (port, enabled, stun) = {
            let saved = self.saved.lock();
            (saved.port, saved.enabled, saved.stun)
        };
        let on = self.running();
        self.live.lock().checking = true;
        let mut probes = self.probes.read().clone();
        if !stun {
            // STUN off (SR1-11): nothing is sent to its servers; the box still says its own address.
            probes.stun.clear();
        }
        let tailscale_probe = self.tailscale_probe.read().clone();
        let server = self.server.load(Ordering::Relaxed);
        // The box is asked to open the port only while the access is on, and never on a server (its firewall).
        let net = nat::probe(&probes, port, on && enabled && !server).await;
        let found = tailscale::detect(&tailscale_probe).await;
        // Without STUN, the box's own address on the Internet, when it is one (not a CGNAT's).
        let public = net
            .public
            .or_else(|| net.wan.filter(|w| !stun && nat::is_public(*w)));
        if let Some(public) = public {
            self.seen(public);
        }
        {
            let mut live = self.live.lock();
            live.net = Some(net);
            live.tailscale = found;
            live.checking = false;
            live.checked_at = Some(now_ms());
        }
        if on {
            self.apply_tailscale().await;
        }
    }

    /// The public address seen: kept, and when it changed, said (the phones paired before it must take the new one).
    fn seen(&self, address: IpAddr) {
        let address = address.to_string();
        let changed = {
            let mut saved = self.saved.lock();
            match saved.public.as_mut() {
                Some(seen) if seen.address == address => false,
                Some(seen) => {
                    log::info!("phone: the public address changed");
                    seen.previous = Some(std::mem::replace(&mut seen.address, address));
                    seen.changed_at = Some(now_ms());
                    seen.since = now_ms();
                    true
                }
                None => {
                    saved.public = Some(Seen {
                        address,
                        since: now_ms(),
                        previous: None,
                        changed_at: None,
                    });
                    true
                }
            }
        };
        if changed {
            if let Err(e) = self.save() {
                log::error!("phone: the public address not kept: {e:#}");
            }
        }
    }

    /// The API served on Tailscale's address when it is there, wanted and the access on; stopped otherwise.
    async fn apply_tailscale(&self) {
        let _one = self.tailscale_lock.lock().await;
        let wanted = self.saved.lock().tailscale;
        let found = self.live.lock().tailscale.clone();
        let port = self.saved.lock().port;
        let (current, api) = match self.run.lock().as_ref() {
            Some(run) => (run.tailscale.as_ref().map(|(a, _)| *a), run.api.clone()),
            None => return,
        };
        let target = match (&found, wanted) {
            (Some(f), true) => Some(SocketAddr::from((f.address, port))),
            _ => None,
        };
        if current == target {
            return;
        }
        // Stopped, or moved.
        if let Some(run) = self.run.lock().as_mut() {
            if let Some((_, task)) = run.tailscale.take() {
                task.abort();
            }
        }
        let (Some(addr), Some(found)) = (target, found) else {
            return;
        };
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                log::warn!("phone: Tailscale's address {addr} does not open: {e}");
                return;
            }
        };
        let mut names = vec![found.address.to_string()];
        if let Some(name) = &found.name {
            names.push(name.to_ascii_lowercase());
        }
        let names = Arc::new(names);
        log::info!("phone: the API served on Tailscale's address {addr}");
        let task = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, from)) => {
                        // A machine of the tailnet alone.
                        if !tailscale::tailnet(from.ip()) {
                            continue;
                        }
                        // Its failures counted by its address (ESC-654).
                        api.serve_stream_from(
                            Box::new(stream),
                            Via::Tailscale(names.clone()),
                            Some(from.ip()),
                        );
                    }
                    Err(e) => {
                        log::debug!("phone: Tailscale's accept: {e}");
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
            }
        });
        if let Some(run) = self.run.lock().as_mut() {
            run.tailscale = Some((addr, task));
        } else {
            task.abort();
        }
    }

    // ---------- what is shown ----------

    /// The access as Réglages › Téléphone shows it.
    pub async fn status(&self, core: &Core) -> Status {
        let tunnel = self.run.lock().as_ref().map(|r| r.tunnel.clone());
        let states = match &tunnel {
            Some(t) => t.peers().await,
            None => Vec::new(),
        };
        let saved = self.saved.lock().clone();
        let live = self.live.lock();
        let run = self.run.lock();
        let denied = core.locks.read().check_phone().err().map(|e| e.to_string());
        let state = if run.is_some() {
            State::On
        } else if live.starting {
            State::Starting
        } else if saved.enabled && (live.error.is_some() || denied.is_some()) {
            State::Error
        } else {
            State::Off
        };
        Status {
            enabled: saved.enabled,
            state,
            error: if saved.enabled {
                live.error.clone()
            } else {
                None
            },
            denied,
            port: saved.port,
            default_port: DEFAULT_PORT,
            bound: run.as_ref().map(|r| r.tunnel.local_addr().to_string()),
            tunnel_address: TUNNEL_ADDRESS.to_string(),
            api: format!("ws://{TUNNEL_ADDRESS}{}", crate::remoteapi::proto::PATH),
            public_key: saved.public_key.clone(),
            server: self.server.load(Ordering::Relaxed),
            net: live.net.clone(),
            checking: live.checking,
            checked_at: live.checked_at,
            public: saved.public.clone(),
            tailscale: live.tailscale.clone(),
            tailscale_on: saved.tailscale,
            tailscale_served: run
                .as_ref()
                .and_then(|r| r.tailscale.as_ref().map(|(a, _)| a.to_string())),
            stun_on: saved.stun,
            stun_servers: nat::Probes::real().stun,
            peers: saved
                .peers
                .iter()
                .map(|p| PeerStatus {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    address: p.address.to_string(),
                    device: p.device.clone(),
                    added_at: p.added_at,
                    since_handshake: states
                        .iter()
                        .find(|s| s.id == p.id)
                        .and_then(|s| s.since_handshake),
                })
                .collect(),
            handshakes: tunnel
                .as_ref()
                .map_or(0, |t| t.counts().handshakes.load(Ordering::Relaxed)),
            unknown: tunnel
                .as_ref()
                .map_or(0, |t| t.counts().unknown.load(Ordering::Relaxed)),
        }
    }

    /// The tunnel, while it runs (the tests read its counts).
    pub fn tunnel(&self) -> Option<Arc<stack::Tunnel>> {
        self.run.lock().as_ref().map(|r| r.tunnel.clone())
    }
}

/// A peer of the tunnel, as the API sees it.
struct PeerLink {
    phone: Weak<Phone>,
    id: String,
}

impl TunnelPeer for PeerLink {
    fn hosts(&self) -> Vec<String> {
        vec![TUNNEL_ADDRESS.to_string()]
    }

    fn device(&self) -> Option<String> {
        let phone = self.phone.upgrade()?;
        let saved = phone.saved.lock();
        saved
            .peers
            .iter()
            .find(|p| p.id == self.id)
            .and_then(|p| p.device.clone())
    }

    fn paired(&self, device: &str) {
        if let Some(phone) = self.phone.upgrade() {
            phone.link(&self.id, device);
        }
    }

    fn label(&self) -> String {
        self.id.clone()
    }
}
