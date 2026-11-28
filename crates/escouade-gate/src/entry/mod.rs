//! The connectors' entry of `escouade-server` (RP-4, `docs/PRIVATE-NETWORK.md` § 4.1, § 4.5, R3): a port apart, off
//! by default, that speaks the connectors' protocol alone, never the remote API (which stays on 127.0.0.1).
//!
//! - **What it serves**: `/connector`, a connector's channel (Noise IK, its key pinned at the pairing), and
//!   `/connector/pair`, the pairing by a code (Noise XXpsk0, `escouade_connector_proto::pairing`). Any other path is a
//!   404, any other speech closed: nothing of the remote API answers here.
//! - **TLS**: with a certificate of its own (`--connector-cert`, `--connector-key`), or plain behind the TLS proxy of a
//!   domain (Traefik, Caddy), the Noise within depending on no certificate.
//! - **Without a key it knows**, a channel is closed after the first Noise message, without a word; a wrong code the
//!   same. Every attempt that fails counts ([`bounds`]): 20 a minute from a source and it is closed to the end of that
//!   minute; 20 a minute from all sources and the pairing is closed to everyone to the end of it; 5 wrong codes from a
//!   source in a code's lifetime and its pairings are refused.
//! - **The channels** go to the [`Registry`], which the gate asks for its flows ([`crate::Connectors`]); a channel cut
//!   makes the gate refuse at once.

pub mod bounds;
pub mod registry;
pub mod store;

pub use bounds::{Bounds, Limits};
/// The protocol of the connectors, for the core (its TLS, its versions).
pub use escouade_connector_proto as proto;
pub use registry::{NewCode, Registry, State, VersionState, View};
pub use store::{MemoryStore, Paired, Pending, Removed, Saved, Store};

use escouade_connector_proto::channel::AcceptError;
use escouade_connector_proto::pairing::{self, CodeKey, PairAcceptError};
use escouade_connector_proto::{accept_connector, Keypair, PublicKey, Version};
use parking_lot::Mutex;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;

/// The time a connection has for its TLS and its WebSocket's upgrade.
pub const UPGRADE_WITHIN: Duration = Duration::from_secs(10);

/// The time a pairing has, its four messages.
pub const PAIR_WITHIN: Duration = Duration::from_secs(30);

/// The connections whose upgrade or handshake is under way at once: past them, a new one is closed.
pub const PENDING_AT_ONCE: usize = 64;

/// What the entry starts with.
pub struct Config {
    /// Where it listens (`--connector-listen`).
    pub listen: SocketAddr,
    /// Its TLS, when it has a certificate of its own.
    pub tls: Option<TlsAcceptor>,
    pub registry: Arc<Registry>,
    /// The proxies whose `X-Forwarded-For` says where a connection comes from (`--trust-proxy`).
    pub proxies: Vec<IpAddr>,
    pub bounds: Limits,
    /// The bounds of each channel.
    pub channel: escouade_connector_proto::Limits,
}

struct Inner {
    keys: Keypair,
    registry: Arc<Registry>,
    proxies: Vec<IpAddr>,
    bounds: Bounds,
    channel: escouade_connector_proto::Limits,
    tls: Option<TlsAcceptor>,
    pending: Arc<tokio::sync::Semaphore>,
    stopped: watch::Receiver<bool>,
}

/// The entry, served until it is stopped or dropped.
pub struct Entry {
    addr: SocketAddr,
    inner: Arc<Inner>,
    stop: watch::Sender<bool>,
    serving: tokio::task::JoinHandle<()>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.stop_now();
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Entry {
    /// The entry on `config.listen`, its key the server's (made the first time).
    pub async fn start(config: Config) -> io::Result<Entry> {
        let keys = config
            .registry
            .server_keys()
            .map_err(|e| io::Error::other(format!("the server's key: {e}")))?;
        let listener = TcpListener::bind(config.listen).await?;
        let addr = listener.local_addr()?;
        let (stop, stopped) = watch::channel(false);
        let inner = Arc::new(Inner {
            keys,
            registry: config.registry,
            proxies: config.proxies,
            bounds: Bounds::new(config.bounds),
            channel: config.channel,
            tls: config.tls,
            pending: Arc::new(tokio::sync::Semaphore::new(PENDING_AT_ONCE)),
            stopped,
        });
        let serving = tokio::spawn(serve(listener, inner.clone()));
        // A connector removed by another process: its channel closed within two seconds.
        {
            let inner = Arc::downgrade(&inner);
            let mut stopped = stop.subscribe();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                        _ = stopped.changed() => return,
                    }
                    let Some(inner) = inner.upgrade() else { return };
                    inner.registry.sync();
                }
            });
        }
        log::info!(
            "connectors' entry on {addr} ({})",
            if inner.tls.is_some() {
                "TLS of its own"
            } else {
                "without TLS: behind a TLS proxy"
            }
        );
        Ok(Entry {
            addr,
            inner,
            stop,
            serving,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn registry(&self) -> &Arc<Registry> {
        &self.inner.registry
    }

    /// The bounds, as the tests look at them.
    pub fn bounds(&self) -> &Bounds {
        &self.inner.bounds
    }

    fn stop_now(&self) {
        let _ = self.stop.send(true);
        self.serving.abort();
        self.inner.registry.close_all();
    }

    /// The entry closed: its port, the handshakes under way, every channel.
    pub fn stop(self) {
        self.stop_now();
        log::info!("connectors' entry on {} closed", self.addr);
    }
}

async fn serve(listener: TcpListener, inner: Arc<Inner>) {
    loop {
        let (s, from) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("connectors' entry: accept: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let source = from.ip().to_string();
        let trusted = inner.proxies.contains(&from.ip());
        // A source past its bounds, closed unread (a trusted proxy's clients are told apart after the headers).
        if !trusted && inner.bounds.closed(&source) {
            continue;
        }
        let Ok(turn) = inner.pending.clone().try_acquire_owned() else {
            log::debug!("connectors' entry: too many handshakes under way, {source} closed");
            continue;
        };
        let inner = inner.clone();
        tokio::spawn(async move {
            let mut stopped = inner.stopped.clone();
            tokio::select! {
                _ = handle(&inner, s, from, turn) => {}
                _ = stopped.changed() => {}
            }
        });
    }
}

/// What the upgrade's request said.
#[derive(Default)]
struct Asked {
    path: String,
    forwarded: Option<IpAddr>,
}

async fn handle(
    inner: &Arc<Inner>,
    s: TcpStream,
    from: SocketAddr,
    turn: tokio::sync::OwnedSemaphorePermit,
) {
    let _ = s.set_nodelay(true);
    let peer = from.ip().to_string();
    match &inner.tls {
        Some(tls) => match tokio::time::timeout(UPGRADE_WITHIN, tls.accept(s)).await {
            Ok(Ok(s)) => upgrade(inner, s, from, turn).await,
            _ => {
                log::debug!("connectors' entry: no TLS from {peer}");
                inner.bounds.failed(&peer);
            }
        },
        None => upgrade(inner, s, from, turn).await,
    }
}

async fn upgrade<S>(
    inner: &Arc<Inner>,
    s: S,
    from: SocketAddr,
    turn: tokio::sync::OwnedSemaphorePermit,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let asked = Arc::new(Mutex::new(Asked::default()));
    let trusted = inner.proxies.contains(&from.ip());
    let pair = pairing::pair_path(escouade_connector_proto::PATH);
    let check = {
        let asked = asked.clone();
        let pair = pair.clone();
        #[allow(clippy::result_large_err)]
        move |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
            let path = req.uri().path().to_string();
            let forwarded = trusted
                .then(|| {
                    req.headers()
                        .get("x-forwarded-for")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.rsplit(',').next())
                        .and_then(|v| v.trim().parse::<IpAddr>().ok())
                })
                .flatten();
            let known = path == escouade_connector_proto::PATH || path == pair;
            *asked.lock() = Asked { path, forwarded };
            if known {
                Ok(resp)
            } else {
                let mut refused = ErrorResponse::new(None);
                *refused.status_mut() = StatusCode::NOT_FOUND;
                Err(refused)
            }
        }
    };
    let upgraded = tokio::time::timeout(
        UPGRADE_WITHIN,
        tokio_tungstenite::accept_hdr_async_with_config(
            s,
            check,
            Some(escouade_connector_proto::transport::ws_config()),
        ),
    )
    .await;
    let source = {
        let a = asked.lock();
        a.forwarded.unwrap_or(from.ip()).to_string()
    };
    let ws = match upgraded {
        Ok(Ok(ws)) => ws,
        _ => {
            // Not a WebSocket of the connectors (a scan, the remote API's client, another path): an attempt that failed.
            log::debug!("connectors' entry: no upgrade from {source}");
            inner.bounds.failed(&source);
            return;
        }
    };
    if inner.bounds.closed(&source) {
        return;
    }
    let path = asked.lock().path.clone();
    if path == pair {
        pairing_of(inner, ws, &source).await;
        drop(turn);
    } else {
        channel_of(inner, ws, &source, turn).await;
    }
}

async fn channel_of<S>(
    inner: &Arc<Inner>,
    ws: tokio_tungstenite::WebSocketStream<S>,
    source: &str,
    turn: tokio::sync::OwnedSemaphorePermit,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let found: Mutex<Option<Paired>> = Mutex::new(None);
    let known = |key: &PublicKey| {
        let c = inner.registry.find(key);
        let ok = c.is_some();
        *found.lock() = c;
        ok
    };
    let version = inner.registry.version();
    let accepted = accept_connector(ws, &inner.keys, known, version, inner.channel).await;
    let found = found.lock().take();
    drop(turn);
    match accepted {
        Ok(channel) => {
            let Some(c) = found else { return };
            let verdict = channel.verdict();
            let peer_version = channel.peer_version();
            let (generation, channel) = inner.registry.attach(&c.id, channel, now_ms());
            log::info!(
                "connectors: “{}” connected (escouade-connector {peer_version}{})",
                c.name,
                if verdict == escouade_connector_proto::Verdict::Outdated {
                    ", to update"
                } else {
                    ""
                }
            );
            follow(inner.registry.clone(), c, generation, channel).await;
        }
        Err(AcceptError::Version { connector, verdict }) => {
            if let Some(c) = found {
                log::warn!(
                    "connectors: “{}” refused: escouade-connector {connector} ({verdict:?}) for a server {version}",
                    c.name
                );
                inner.registry.refused_version(&c.id, connector, now_ms());
            }
        }
        Err(e) if e.is_failed_attempt() => {
            log::debug!("connectors' entry: {source}: {e}");
            inner.bounds.failed(source);
        }
        Err(e) => log::debug!("connectors' entry: {source}: {e}"),
    }
}

/// A channel followed to its end: its ceiling kept as it comes, its cut said.
async fn follow(
    registry: Arc<Registry>,
    c: Paired,
    generation: u64,
    channel: Arc<escouade_connector_proto::ServerChannel>,
) {
    let mut ceiling = channel.watch_ceiling();
    if let Some(now) = ceiling.borrow_and_update().clone() {
        registry.ceiling_received(&c.id, generation, now.rules);
    }
    loop {
        tokio::select! {
            changed = ceiling.changed() => {
                if changed.is_err() {
                    break;
                }
                let rules = ceiling.borrow_and_update().clone();
                if let Some(rules) = rules {
                    registry.ceiling_received(&c.id, generation, rules.rules);
                }
            }
            why = channel.closed() => {
                log::info!("connectors: “{}” cut: {why}", c.name);
                break;
            }
        }
    }
    registry.detach(&c.id, generation, now_ms());
}

async fn pairing_of<S>(inner: &Arc<Inner>, ws: tokio_tungstenite::WebSocketStream<S>, source: &str)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    if !inner.bounds.may_pair(source) {
        log::debug!("connectors' entry: a pairing from {source} refused unread (its bounds)");
        return;
    }
    let now = now_ms();
    let pending = inner.registry.pending(now);
    // A pending code whose key is unreadable is left out: the indexes follow `keys`.
    let usable: Vec<(Pending, CodeKey)> = pending
        .into_iter()
        .filter_map(|p| {
            let bytes: [u8; 32] = hex::decode(&p.key).ok()?.try_into().ok()?;
            Some((p, CodeKey::from_bytes(bytes)))
        })
        .collect();
    let keys: Vec<CodeKey> = usable.iter().map(|(_, k)| k.clone()).collect();
    let registry = inner.registry.clone();
    let decide = |index: usize, key: &PublicKey, version: Version| {
        let (p, _) = usable
            .get(index)
            .ok_or_else(|| "no such code".to_string())?;
        registry.pin(p, key, version, now_ms()).map(|_| ())
    };
    let version = registry.version();
    match pairing::accept_pairing(ws, &inner.keys, version, &keys, decide, PAIR_WITHIN).await {
        Ok(accepted) => log::info!(
            "connectors: a pairing from {source} (escouade-connector {})",
            accepted.version
        ),
        Err(PairAcceptError::WrongCode) => {
            log::info!("connectors' entry: a wrong pairing code from {source}");
            inner.bounds.wrong_code(source);
        }
        Err(e) if e.is_failed_attempt() => {
            log::debug!("connectors' entry: {source}: {e}");
            inner.bounds.failed(source);
        }
        Err(e) => log::info!("connectors' entry: a pairing from {source}: {e}"),
    }
}
