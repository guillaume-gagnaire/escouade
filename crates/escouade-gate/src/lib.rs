//! The gate of `escouade-server` (RP-1, `docs/PRIVATE-NETWORK.md` § 4.3; spec « Escouade à distance › Atteindre un
//! réseau privé »): the one proxy of the server's agents, on 127.0.0.1, by which every connection they open goes out.
//!
//! - **One port, two tongues**: HTTP (`CONNECT host:port`, and a request in plain HTTP in its absolute form) and SOCKS5
//!   by name (`socks5h`), told apart by their first byte.
//! - **A token per agent** in the proxy's address (`http://<token>@127.0.0.1:P`, `socks5h://<token>@…`): it says the
//!   agent and its project. None, or one the gate did not give, is refused.
//! - **The routes** ([`decide`]): a host of the project's list goes to the connector of its network (R12); a host of a
//!   private network out of the list (a connector's ceiling, another project's list) is refused; the rest goes out
//!   straight, or through the space's proxy, which stands behind the gate ([`Outside`]).
//! - **The refusals** read as sentences ([`Reason`]): the `403` of a `CONNECT`, the SOCKS code « connection not allowed
//!   by ruleset », a line on the standard error of `escouade-server via` for `ssh`. A connector cut refuses at once.
//! - **The journal** of each connection, allowed or refused, never what went through ([`journal`]).
//!
//! A consigne, not a wall (§ 4.3, SEC-02): every agent of the server runs under the same account and may ignore the
//! variables; what holds is the connector's ceiling, in the company's network.

pub mod connectors;
pub mod entry;
pub mod env;
mod http;
pub mod journal;
pub mod lists;
pub mod outside;
pub mod pipe;
pub mod reason;
mod socks;
pub mod via;

pub use connectors::{Ask, Connectors, Failed, Flow, Known, NoConnectors, Reach};
pub use journal::{Entry, Route};
pub use lists::{Allowed, Lists, Pattern, ProjectList, Target};
pub use outside::Outside;
pub use reason::{Lang, Reason};

use parking_lot::{Mutex, RwLock};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;

/// The connections open at once, beyond which the gate waits.
pub const AT_ONCE: usize = 1024;

/// The time a connector has to open a flow.
pub const OPEN_WITHIN: Duration = Duration::from_secs(30);

/// Who holds a token: an agent, or what else of a project goes through the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    /// The agent's id, or `terminal`, `launch`, `git`.
    pub who: String,
    /// The project's id.
    pub project: String,
    /// The project's name, for the refusals.
    pub project_name: String,
}

/// What the gate starts with.
pub struct Config {
    pub lang: Lang,
    pub connectors: Arc<dyn Connectors>,
    pub lists: Lists,
    pub outside: Outside,
    /// Its port on 127.0.0.1; 0 to let the system choose.
    pub port: u16,
}

struct Inner {
    lang: Lang,
    connectors: Arc<dyn Connectors>,
    lists: RwLock<Lists>,
    outside: RwLock<Outside>,
    /// The holders, by the SHA-256 of their token: the token itself is kept nowhere.
    tokens: RwLock<HashMap<[u8; 32], Holder>>,
    journal: Mutex<journal::Journal>,
    at_once: Semaphore,
}

/// The gate, served until it is dropped or stopped.
#[derive(Clone)]
pub struct Gate {
    inner: Arc<Inner>,
    addr: SocketAddr,
    serving: Arc<Serving>,
}

struct Serving(tokio::task::JoinHandle<()>);

impl Drop for Serving {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// Where a connection of a project goes, from its list and its connectors (pure: the tests call it as it is).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Through the connector `id`, of that name.
    Connector {
        id: String,
        name: String,
    },
    /// Out, straight or through the space's proxy.
    Outside,
    Refused(Reason),
}

/// The route of `t` for `holder`: the project's list first, then the private networks it is out of, then outside.
pub fn decide(lists: &Lists, known: &[Known], holder: &Holder, t: &Target) -> Decision {
    if let Some(allowed) = lists.find(&holder.project, t) {
        let Some(c) = known.iter().find(|c| c.id == allowed.connector) else {
            return Decision::Refused(Reason::UnknownConnector {
                target: t.to_string(),
                connector: allowed.connector.clone(),
            });
        };
        if let Reach::Down { since_ms } = c.reach {
            return Decision::Refused(Reason::Down {
                connector: c.name.clone(),
                since_ms,
            });
        }
        // Its ceiling known: what is out of it is refused here, at once; else the connector says.
        if !c.ceiling.is_empty() && !c.ceiling.iter().any(|p| p.matches(t)) {
            return Decision::Refused(Reason::NotInCeiling {
                target: t.to_string(),
                connector: c.name.clone(),
            });
        }
        return Decision::Connector {
            id: c.id.clone(),
            name: c.name.clone(),
        };
    }
    // A name of a private network, on any port: never resolved here, never out.
    let private = lists.names(&t.host)
        || known
            .iter()
            .any(|c| c.ceiling.iter().any(|p| p.names(&t.host)));
    if private {
        return Decision::Refused(Reason::NotInList {
            target: t.to_string(),
            project: holder.project_name.clone(),
        });
    }
    Decision::Outside
}

/// A connection asked, as the journal will say it.
pub(crate) struct Asked {
    pub at: u64,
    pub started: Instant,
    pub by: &'static str,
    pub target: Option<Target>,
    pub holder: Option<Holder>,
}

impl Asked {
    pub fn new(by: &'static str) -> Self {
        Self {
            at: now_ms(),
            started: Instant::now(),
            by,
            target: None,
            holder: None,
        }
    }
}

/// Where a flow was opened.
pub(crate) struct Reached {
    pub flow: Flow,
    pub route: Route,
    pub connector: Option<String>,
    /// For a request in plain HTTP sent to the space's proxy: its authorization (`Some(None)` when it has none).
    pub upstream_auth: Option<Option<String>>,
}

impl Inner {
    fn holder(&self, token: &str) -> Option<Holder> {
        self.tokens.read().get(&digest(token)).cloned()
    }

    /// The holder of `token`, or why not.
    pub(crate) fn admit(&self, token: Option<&str>) -> Result<Holder, Reason> {
        match token.filter(|t| !t.is_empty()) {
            None => Err(Reason::NoToken),
            Some(t) => self.holder(t).ok_or(Reason::UnknownToken),
        }
    }

    /// A flow to `t` for `holder`, by its route; `plain` for a request in plain HTTP.
    pub(crate) async fn reach(
        &self,
        holder: &Holder,
        t: &Target,
        plain: bool,
    ) -> Result<Reached, Reason> {
        let known = self.connectors.known();
        let decision = decide(&self.lists.read(), &known, holder, t);
        match decision {
            Decision::Refused(r) => Err(r),
            Decision::Connector { id, name } => {
                let ask = Ask {
                    host: t.host.clone(),
                    port: t.port,
                    agent: holder.who.clone(),
                    project: holder.project.clone(),
                };
                let opened = tokio::time::timeout(OPEN_WITHIN, self.connectors.open(&id, ask))
                    .await
                    .unwrap_or_else(|_| Err(Failed::Broken("timed out".into())));
                match opened {
                    Ok(flow) => Ok(Reached {
                        flow,
                        route: Route::Connector,
                        connector: Some(id),
                        upstream_auth: None,
                    }),
                    Err(Failed::Refused(why)) => Err(Reason::RefusedByConnector {
                        target: t.to_string(),
                        connector: name,
                        why,
                    }),
                    Err(Failed::Down { since_ms }) => Err(Reason::Down {
                        connector: name,
                        since_ms,
                    }),
                    Err(Failed::Broken(why)) => Err(Reason::Unreachable {
                        target: t.to_string(),
                        why,
                    }),
                }
            }
            Decision::Outside => {
                let outside = self.outside.read().clone();
                if plain {
                    let (flow, route, auth) = outside.plain(t).await?;
                    Ok(Reached {
                        flow,
                        route,
                        connector: None,
                        upstream_auth: auth,
                    })
                } else {
                    let (flow, route) = outside.tunnel(t).await?;
                    Ok(Reached {
                        flow,
                        route,
                        connector: None,
                        upstream_auth: None,
                    })
                }
            }
        }
    }

    fn entry(&self, asked: &Asked) -> Entry {
        let (host, port) = asked
            .target
            .as_ref()
            .map(|t| (t.host.clone(), t.port))
            .unwrap_or_default();
        let (agent, project) = asked
            .holder
            .as_ref()
            .map(|h| (h.who.clone(), h.project.clone()))
            .unwrap_or_default();
        Entry {
            at: asked.at,
            host,
            port,
            agent,
            project,
            by: asked.by,
            route: Route::Refused,
            connector: None,
            allowed: false,
            refusal: None,
            reason: None,
            up: 0,
            down: 0,
            ms: asked.started.elapsed().as_millis() as u64,
        }
    }

    /// A refused (or failed) connection, in the journal.
    pub(crate) fn refused(&self, asked: &Asked, why: &Reason) {
        let mut e = self.entry(asked);
        e.refusal = Some(why.kind());
        e.reason = Some(why.say(self.lang));
        self.journal.lock().push(e);
    }

    /// An allowed connection, ended, in the journal.
    pub(crate) fn ended(&self, asked: &Asked, reached: (Route, Option<String>), bytes: (u64, u64)) {
        let mut e = self.entry(asked);
        e.allowed = true;
        e.route = reached.0;
        e.connector = reached.1;
        e.up = bytes.0;
        e.down = bytes.1;
        self.journal.lock().push(e);
    }
}

impl Gate {
    /// The gate on 127.0.0.1, served on the runtime it is started on.
    pub async fn start(config: Config) -> std::io::Result<Self> {
        let listener =
            TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, config.port))).await?;
        let addr = listener.local_addr()?;
        let inner = Arc::new(Inner {
            lang: config.lang,
            connectors: config.connectors,
            lists: RwLock::new(config.lists.shaped()),
            outside: RwLock::new(config.outside),
            tokens: RwLock::default(),
            journal: Mutex::default(),
            at_once: Semaphore::new(AT_ONCE),
        });
        let serving = tokio::spawn(serve(listener, Arc::downgrade(&inner)));
        log::info!("gate: on {addr}");
        Ok(Self {
            inner,
            addr,
            serving: Arc::new(Serving(serving)),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// A new token for `holder`: 32 random bytes, in hexadecimal. Only its digest is kept.
    pub fn issue(&self, holder: Holder) -> String {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("the system's random generator");
        let token = hex::encode(bytes);
        self.inner.tokens.write().insert(digest(&token), holder);
        token
    }

    /// `token` taken back.
    pub fn revoke(&self, token: &str) {
        self.inner.tokens.write().remove(&digest(token));
    }

    /// Every token of `who` taken back.
    pub fn revoke_who(&self, who: &str) {
        self.inner.tokens.write().retain(|_, h| h.who != who);
    }

    /// The holder of `token`.
    pub fn holder(&self, token: &str) -> Option<Holder> {
        self.inner.holder(token)
    }

    /// The lists of the projects, read again.
    pub fn set_lists(&self, lists: Lists) {
        *self.inner.lists.write() = lists.shaped();
    }

    /// The way out, when the space's proxy changed.
    pub fn set_outside(&self, outside: Outside) {
        *self.inner.outside.write() = outside;
    }

    /// The last connections, the oldest first.
    pub fn journal(&self) -> Vec<Entry> {
        self.inner.journal.lock().entries()
    }

    /// The proxy's address for `token`, in HTTP.
    pub fn http_url(&self, token: &str) -> String {
        format!("http://{token}@127.0.0.1:{}", self.addr.port())
    }

    /// The proxy's address for `token`, in SOCKS5 by name.
    pub fn socks_url(&self, token: &str) -> String {
        format!("socks5h://{token}@127.0.0.1:{}", self.addr.port())
    }

    /// What a process given `token` goes out with (`env::gate_env`).
    pub fn env(&self, token: &str, ssh_command: &str) -> Vec<(String, String)> {
        env::gate_env(&self.http_url(token), &self.socks_url(token), ssh_command)
    }

    /// The gate closed: its port, and what it serves.
    pub fn stop(&self) {
        self.serving.0.abort();
    }
}

async fn serve(listener: TcpListener, inner: std::sync::Weak<Inner>) {
    loop {
        let (s, from) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("gate: accept: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let Some(inner) = inner.upgrade() else { return };
        if !from.ip().is_loopback() {
            continue;
        }
        tokio::spawn(async move {
            let Ok(_turn) = inner.at_once.acquire().await else {
                return;
            };
            handle(&inner, s).await;
        });
    }
}

async fn handle(inner: &Inner, s: TcpStream) {
    let _ = s.set_nodelay(true);
    let mut first = [0u8; 1];
    let peeked = tokio::time::timeout(pipe::HEAD_WITHIN, s.peek(&mut first)).await;
    match peeked {
        Ok(Ok(1)) if first[0] == 5 => socks::serve(inner, s).await,
        Ok(Ok(1)) => http::serve(inner, s).await,
        _ => {}
    }
}
