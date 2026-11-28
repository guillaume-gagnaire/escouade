//! The service (`escouade-connector run`, § 4.8): the channel to the server, opened again on the schedule after a cut;
//! each `open` checked ([`crate::guard`]), joined, relayed and journaled; `connector.toml` and the pairing read again
//! while it runs.
//!
//! - **The schedule** (§ 4.2): 250 ms, 500 ms, 1 s, 2 s, 4 s, then every 5 s ([`Backoff`]), back to 250 ms once a
//!   channel is up. A server that no longer knows this connector, or refuses its version, is tried again every
//!   [`SLOW_RETRY`] (each try counts as a failed attempt on its side), the journal saying what to do.
//! - **A cut**: the streams open are closed, none taken up again (a `git fetch` starts again; a TCP does not resume).
//! - **A change of `connector.toml`**: said in the journal; the ceiling sent again to the server; a stream whose
//!   host or address the new ceiling no longer lets through closed; another server, proxy or authorities: the channel
//!   opened again. A file that no longer reads: said, and the last one read is kept.
//! - **A change of the pairing** (`pair` or `unpair` beside the service): the channel closed and opened again with the
//!   new keys, or not at all.
//! - **The bounds**: the streams open at once and the opens a second of `connector.toml`, refused here and journaled;
//!   a stream silent for its `idle` closed by the channel.

use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use escouade_connector_proto::life::Backoff;
use escouade_connector_proto::transport::{self, ClientConfig, Proxy};
use escouade_connector_proto::{
    connect_server, Ceiling as CeilingFrame, ConnectError, Incoming, Limits, Refusal, RefusalCode,
    Version,
};
use serde_json::json;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_tungstenite::WebSocketStream;

use crate::config::{self, Config, MAX_OPENS_PER_SECOND, MAX_STREAMS};
use crate::guard::{self, Guards, Resolve};
use crate::journal::Journal;
use crate::keys::{Pairing, State};

/// How often `connector.toml` and the pairing are read again.
pub const RELOAD_EVERY: Duration = Duration::from_secs(2);
/// The tries after a refusal by the server (not paired, a version refused).
pub const SLOW_RETRY: Duration = Duration::from_secs(60);

/// How the channel's WebSocket is opened: [`Wss`] for real, a fake in the tests.
pub trait Dial: Send + Sync + 'static {
    type Io: AsyncRead + AsyncWrite + Unpin + Send + 'static;
    fn dial(
        &self,
        config: &ClientConfig,
    ) -> impl Future<Output = Result<WebSocketStream<Self::Io>, ConnectError>> + Send;
}

/// The WebSocket over TLS, through the company's proxy (the protocol's [`transport::connect`]).
pub struct Wss;

impl Dial for Wss {
    type Io = tokio_rustls::client::TlsStream<TcpStream>;
    async fn dial(&self, config: &ClientConfig) -> Result<WebSocketStream<Self::Io>, ConnectError> {
        transport::connect(config).await
    }
}

/// What the service is given.
pub struct Options {
    pub config_path: PathBuf,
    pub state: State,
    pub journal: Journal,
    pub resolver: Arc<dyn Resolve>,
    pub guards: Guards,
    pub version: Version,
    /// The system's authorities and Mozilla's for the outer TLS (`false` in the tests: the file alone).
    pub system_roots: bool,
    pub reload_every: Duration,
}

/// The way to the server for a config and a pairing.
pub fn client_config(config: &Config, pairing: &Pairing, system_roots: bool) -> ClientConfig {
    let mut c = ClientConfig::new(pairing.server.clone());
    c.system_roots = system_roots;
    c.authorities = config.authorities.clone();
    c.proxy = match &config.proxy {
        Some(p) => Some(p.clone()),
        None => transport::parse_target(&pairing.server)
            .ok()
            .and_then(|t| Proxy::from_env(&t.host))
            .and_then(Result::ok),
    };
    c
}

fn proxy_shown(c: &ClientConfig) -> Option<String> {
    c.proxy.as_ref().map(|p| format!("{}:{}", p.host, p.port))
}

/// What the reloading task publishes.
#[derive(Clone)]
struct Watched {
    config: watch::Receiver<Arc<Config>>,
    /// Bumped at each change of the pairing's files.
    pairing: watch::Receiver<u64>,
}

/// Reads `connector.toml` and the state every `every`, and publishes what changed.
fn spawn_reloader(
    opts: &Options,
    first: Arc<Config>,
    first_bytes: Vec<u8>,
) -> (Watched, tokio::task::JoinHandle<()>) {
    let (config_tx, config) = watch::channel(first);
    let (pairing_tx, pairing) = watch::channel(0u64);
    let path = opts.config_path.clone();
    let state = opts.state.clone();
    let journal = opts.journal.clone();
    let every = opts.reload_every;
    let task = tokio::spawn(async move {
        let mut bytes = first_bytes;
        let mut bad: Option<Vec<u8>> = None;
        let mut stamp = state.stamp();
        let mut tick = tokio::time::interval(every);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        tick.tick().await;
        loop {
            tick.tick().await;
            if config_tx.is_closed() {
                return;
            }
            let now = state.stamp();
            if now != stamp {
                stamp = now;
                pairing_tx.send_modify(|g| *g += 1);
            }
            let Ok(read) = std::fs::read(&path) else {
                if bad.as_deref() != Some(&[][..]) {
                    bad = Some(Vec::new());
                    journal.write(
                        "config-error",
                        json!({"path": path.display().to_string(), "message": "connector.toml cannot be read: the last one read is kept"}),
                    );
                }
                continue;
            };
            if read == bytes || bad.as_ref() == Some(&read) {
                continue;
            }
            let text = String::from_utf8_lossy(&read).into_owned();
            match config::parse(&text, &path) {
                Ok(new) => {
                    let old = config_tx.borrow().clone();
                    bytes = read;
                    bad = None;
                    if *old == new {
                        continue;
                    }
                    say_change(&journal, &old, &new);
                    let _ = config_tx.send(Arc::new(new));
                }
                Err(e) => {
                    bad = Some(read);
                    journal.write(
                        "config-error",
                        json!({"message": format!("{e}: the last one read is kept")}),
                    );
                }
            }
        }
    });
    (Watched { config, pairing }, task)
}

fn say_change(journal: &Journal, old: &Config, new: &Config) {
    let (was, is) = (old.ceiling.lines(), new.ceiling.lines());
    let added: Vec<_> = is.iter().filter(|l| !was.contains(l)).cloned().collect();
    let removed: Vec<_> = was.iter().filter(|l| !is.contains(l)).cloned().collect();
    let mut changed = Vec::new();
    if !added.is_empty() || !removed.is_empty() {
        changed.push("ceiling");
    }
    if old.bounds != new.bounds {
        changed.push("limits");
    }
    if old.way_differs(new) {
        changed.push("server");
    }
    if old.auto_update != new.auto_update {
        changed.push("auto_update");
    }
    journal.write(
        "config-reloaded",
        json!({
            "changed": changed,
            "added": added,
            "removed": removed,
            "ceiling": is,
            "max_streams": new.bounds.max_streams,
            "opens_per_second": new.bounds.opens_per_second,
            "idle_minutes": new.bounds.idle.as_secs() / 60,
            "message": if old.bounds.idle != new.bounds.idle {
                Some("the new idle bound holds from the next connection to the server")
            } else {
                None
            },
        }),
    );
}

/// The opens of the last second, as the protocol counts them (refused ones included).
#[derive(Default)]
struct Rate {
    second: Option<Instant>,
    count: u32,
}

impl Rate {
    fn admit(&mut self, now: Instant, per_second: u32) -> bool {
        match self.second {
            Some(start) if now.duration_since(start) < Duration::from_secs(1) => {}
            _ => {
                self.second = Some(now);
                self.count = 0;
            }
        }
        self.count += 1;
        self.count <= per_second
    }
}

/// What the streams of the service share.
struct Shared {
    journal: Journal,
    resolver: Arc<dyn Resolve>,
    guards: Guards,
    config: watch::Receiver<Arc<Config>>,
    active: AtomicUsize,
    rate: Mutex<Rate>,
}

struct Active<'a>(&'a AtomicUsize);

impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn code_name(code: RefusalCode) -> &'static str {
    match code {
        RefusalCode::NotAllowed => "not-allowed",
        RefusalCode::Unresolved => "unresolved",
        RefusalCode::Unreachable => "unreachable",
        RefusalCode::Limits => "limits",
        RefusalCode::Closing => "closing",
        RefusalCode::Other(_) => "other",
    }
}

/// One `open`: the bounds, the checks, the connection, the relay, each said in the journal.
async fn serve_open(shared: Arc<Shared>, incoming: Incoming) {
    let open = incoming.open().clone();
    let started = Instant::now();
    let config = shared.config.borrow().clone();
    let refuse = |incoming: Incoming, refusal: Refusal, address: Option<String>| {
        shared.journal.write(
            "refused",
            json!({
                "host": open.host,
                "port": open.port,
                "address": address,
                "agent": open.agent,
                "project": open.project,
                "code": code_name(refusal.code),
                "reason": refusal.reason,
            }),
        );
        incoming.refuse(refusal);
    };
    let admitted_now = shared
        .rate
        .lock()
        .unwrap()
        .admit(Instant::now(), config.bounds.opens_per_second);
    let active = shared.active.fetch_add(1, Ordering::SeqCst);
    let _active = Active(&shared.active);
    if !admitted_now || active >= config.bounds.max_streams {
        return refuse(
            incoming,
            Refusal::new(
                RefusalCode::Limits,
                format!(
                    "the connector holds {} streams at most, {} opened a second",
                    config.bounds.max_streams, config.bounds.opens_per_second
                ),
            ),
            None,
        );
    }
    let admitted = match guard::admit(
        &open.host,
        open.port,
        &config.ceiling,
        shared.resolver.as_ref(),
        &shared.guards,
    )
    .await
    {
        Ok(a) => a,
        Err(refusal) => return refuse(incoming, refusal, None),
    };
    let (tcp, addr) = match guard::join(&admitted, open.port, guard::CONNECT_TIMEOUT).await {
        Ok(joined) => joined,
        Err(refusal) => {
            let tried = admitted
                .addrs
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return refuse(incoming, refusal, Some(tried));
        }
    };
    let dropped: Vec<String> = admitted
        .dropped
        .iter()
        .map(|(ip, why)| format!("{ip} ({why})"))
        .collect();
    shared.journal.write(
        "allowed",
        json!({
            "host": admitted.host,
            "port": open.port,
            "address": addr.ip().to_string(),
            "agent": open.agent,
            "project": open.project,
            "addresses_refused": if dropped.is_empty() { None } else { Some(dropped) },
        }),
    );
    let mut stream = incoming.accept();
    let mut tcp = tcp;
    let mut config_rx = shared.config.clone();
    let host = admitted.host.clone();
    let ip = addr.ip();
    let guards = shared.guards;
    let relay = tokio::io::copy_bidirectional(&mut stream, &mut tcp);
    tokio::pin!(relay);
    // Not let through any more by a new ceiling: closed.
    let still_allowed = async {
        loop {
            if config_rx.changed().await.is_err() {
                return std::future::pending::<String>().await;
            }
            let c = config_rx.borrow_and_update().clone();
            let ceiling_says = c.ceiling.admit(&host, open.port);
            let refused = match ceiling_says {
                crate::ceiling::Admission::Refused(why) => Some(why),
                _ => guard::refuse_address(ip, &c.ceiling, &guards),
            };
            if let Some(why) = refused {
                return why;
            }
        }
    };
    let (sent, received, reason) = tokio::select! {
        r = &mut relay => match r {
            Ok((to_host, from_host)) => (Some(to_host), Some(from_host), None),
            Err(e) => (None, None, Some(format!("the stream ended: {e}"))),
        },
        why = still_allowed => (None, None, Some(format!("closed: the ceiling no longer lets it through ({why})"))),
    };
    shared.journal.write(
        "closed",
        json!({
            "host": admitted.host,
            "port": open.port,
            "address": ip.to_string(),
            "agent": open.agent,
            "project": open.project,
            "bytes_to_host": sent,
            "bytes_from_host": received,
            "duration_ms": started.elapsed().as_millis() as u64,
            "reason": reason,
        }),
    );
}

/// Why [`run`] stops before it serves.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct RunError(pub String);

/// Resolves once `stop` is set (never when its sender is gone).
async fn stopped(stop: &mut watch::Receiver<bool>) {
    loop {
        if *stop.borrow_and_update() {
            return;
        }
        if stop.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// Waits `delay`, or less when something changed or the service is stopped. `true` when stopped.
async fn pause(delay: Duration, w: &mut Watched, stop: &mut watch::Receiver<bool>) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(delay) => false,
        _ = w.pairing.changed() => false,
        _ = stopped(stop) => true,
    }
}

/// The service, until `stop` is set: `Ok` when stopped, `Err` when `connector.toml` does not read at the start.
pub async fn run<D: Dial>(
    opts: Options,
    dial: D,
    mut stop: watch::Receiver<bool>,
) -> Result<(), RunError> {
    let (first, bytes) = config::load(&opts.config_path).map_err(|e| RunError(e.to_string()))?;
    let journal = opts.journal.clone();
    journal.write(
        "started",
        json!({
            "version": opts.version.to_string(),
            "config": opts.config_path.display().to_string(),
            "state": opts.state.dir.display().to_string(),
            "ceiling": first.ceiling.lines(),
            "max_streams": first.bounds.max_streams,
            "opens_per_second": first.bounds.opens_per_second,
            "idle_minutes": first.bounds.idle.as_secs() / 60,
        }),
    );
    if first.auto_update {
        journal.write(
            "auto-update",
            json!({"message": "auto_update = true is read; this version does not update itself yet: escouade-connector update"}),
        );
    }
    let (mut watched, reloader) = spawn_reloader(&opts, Arc::new(first), bytes);
    let shared = Arc::new(Shared {
        journal: journal.clone(),
        resolver: opts.resolver.clone(),
        guards: opts.guards,
        config: watched.config.clone(),
        active: AtomicUsize::new(0),
        rate: Mutex::new(Rate::default()),
    });
    let mut backoff = Backoff::new();
    let mut said_unpaired = false;
    let mut said_mismatch: Option<String> = None;
    let outcome = loop {
        if *stop.borrow() {
            break Ok(());
        }
        let pairing = match opts.state.load() {
            Ok(Some(p)) => p,
            Ok(None) => {
                if !said_unpaired {
                    said_unpaired = true;
                    journal.write(
                        "not-paired",
                        json!({"message": "this connector is not paired: escouade-connector pair <address> <code> (the code from Space settings › Private network)"}),
                    );
                }
                if pause(Duration::from_secs(3600), &mut watched, &mut stop).await {
                    break Ok(());
                }
                continue;
            }
            Err(e) => {
                journal.write("state-error", json!({"message": e.to_string()}));
                if pause(SLOW_RETRY, &mut watched, &mut stop).await {
                    break Ok(());
                }
                continue;
            }
        };
        if said_unpaired {
            said_unpaired = false;
            journal.write(
                "paired",
                json!({"server": pairing.0.server, "connector_key": pairing.0.connector_key.to_hex()}),
            );
        }
        let (pairing, keys) = pairing;
        journal.forbid(hex::encode(keys.private_bytes()));
        let config = watched.config.borrow().clone();
        if let Some(server) = &config.server {
            if server.trim_end_matches('/') != pairing.server.trim_end_matches('/') {
                if said_mismatch.as_deref() != Some(server) {
                    said_mismatch = Some(server.clone());
                    journal.write(
                        "server-mismatch",
                        json!({"configured": server, "paired": pairing.server, "message": "connector.toml names another server than the one paired: change server, or unpair and pair again"}),
                    );
                }
                let mut config_rx = watched.config.clone();
                tokio::select! {
                    _ = config_rx.changed() => {}
                    _ = watched.pairing.changed() => {}
                    _ = stopped(&mut stop) => break Ok(()),
                }
                continue;
            }
        }
        said_mismatch = None;
        let client = client_config(&config, &pairing, opts.system_roots);
        let ws = tokio::select! {
            ws = dial.dial(&client) => ws,
            _ = stopped(&mut stop) => break Ok(()),
        };
        let ws = match ws {
            Ok(ws) => ws,
            Err(e) => {
                let delay = backoff.next_delay();
                journal.write(
                    "unreachable",
                    json!({"server": pairing.server, "proxy": proxy_shown(&client), "message": e.to_string(), "retry_in_ms": delay.as_millis() as u64}),
                );
                if pause(delay, &mut watched, &mut stop).await {
                    break Ok(());
                }
                continue;
            }
        };
        let limits = Limits {
            max_streams: MAX_STREAMS,
            opens_per_second: MAX_OPENS_PER_SECOND,
            stream_idle: config.bounds.idle,
            ..Limits::default()
        };
        let frame = CeilingFrame {
            rules: config.ceiling.lines(),
        };
        let channel =
            connect_server(ws, &keys, &pairing.server_key, opts.version, &frame, limits).await;
        let mut channel = match channel {
            Ok(c) => c,
            Err(e) => {
                let (event, delay) = match &e {
                    ConnectError::NotPaired => ("not-known", SLOW_RETRY),
                    ConnectError::NotTheServer => ("not-the-server", SLOW_RETRY),
                    ConnectError::Version { .. } => ("version-refused", SLOW_RETRY),
                    _ => ("unreachable", backoff.next_delay()),
                };
                let message = match &e {
                    ConnectError::NotPaired => "the server closed at the handshake: it does not know this connector (removed from the space?): unpair, then pair again with a new code".to_string(),
                    other => other.to_string(),
                };
                let verdict = match &e {
                    ConnectError::Version { welcome, .. } => Some(format!("{:?}", welcome.verdict)),
                    _ => None,
                };
                journal.write(
                    event,
                    json!({"server": pairing.server, "message": message, "verdict": verdict, "retry_in_ms": delay.as_millis() as u64}),
                );
                if pause(delay, &mut watched, &mut stop).await {
                    break Ok(());
                }
                continue;
            }
        };
        backoff.reset();
        let welcome = channel.welcome();
        journal.write(
            "connected",
            json!({
                "server": pairing.server,
                "proxy": proxy_shown(&client),
                "server_version": welcome.version.to_string(),
                "verdict": format!("{:?}", welcome.verdict),
                "ceiling": config.ceiling.lines(),
            }),
        );
        if let Some(advice) = welcome.advice(opts.version) {
            journal.write("outdated", json!({"message": advice}));
        }
        // Serve until the channel ends, the way or the pairing changes, or the service stops.
        let mut config_rx = watched.config.clone();
        config_rx.mark_unchanged();
        let mut streams = tokio::task::JoinSet::new();
        let why = loop {
            tokio::select! {
                incoming = channel.next_open() => match incoming {
                    Some(incoming) => {
                        streams.spawn(serve_open(shared.clone(), incoming));
                    }
                    None => break channel.closed().await.to_string(),
                },
                changed = config_rx.changed() => {
                    if changed.is_err() {
                        continue;
                    }
                    let new = config_rx.borrow_and_update().clone();
                    if new.way_differs(&config) {
                        channel.close();
                        break "connector.toml changed the way to the server: opened again".into();
                    }
                    let frame = CeilingFrame { rules: new.ceiling.lines() };
                    if let Err(e) = channel.send_ceiling(&frame) {
                        journal.write("config-error", json!({"message": format!("the ceiling is not sent to the server: {e}")}));
                    }
                }
                _ = watched.pairing.changed() => {
                    channel.close();
                    break "the pairing changed on this machine".into();
                }
                _ = stopped(&mut stop) => {
                    channel.close();
                    break "the service is stopping".into();
                }
            }
        };
        // The streams open are closed, none taken up again.
        // The channel's end resets them: each says its end in the journal; what still hangs is cut.
        drop(channel);
        let ended = tokio::time::timeout(Duration::from_secs(5), async {
            while streams.join_next().await.is_some() {}
        })
        .await;
        if ended.is_err() {
            streams.abort_all();
            while streams.join_next().await.is_some() {}
        }
        let stopping = *stop.borrow();
        let delay = if stopping {
            Duration::ZERO
        } else {
            backoff.next_delay()
        };
        journal.write(
            "cut",
            json!({"server": pairing.server, "message": why, "retry_in_ms": if stopping { None } else { Some(delay.as_millis() as u64) }}),
        );
        if stopping {
            break Ok(());
        }
        if pause(delay, &mut watched, &mut stop).await {
            break Ok(());
        }
    };
    reloader.abort();
    journal.write("stopped", json!({}));
    outcome
}
