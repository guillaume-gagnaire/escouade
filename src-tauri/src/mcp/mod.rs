//! The MCP server (« Claude peut piloter Escouade »): Claude Code, in a terminal, another tool or
//! an agent of Escouade, reads the projects, agents and tickets and acts on them through the
//! tools of `tools`. On HTTP, on 127.0.0.1 only (`http`: hyper, guards of its own, then the
//! official SDK `rmcp`), each request with a bearer token: the one for Claude outside Escouade
//! (in the keychain, written into Claude Code's config) or an agent's (`register_agent`, in
//! memory only). It runs while the settings want it (`Core::sync_mcp`) and stops with the app.

pub(crate) mod activity;
mod http;
#[cfg(test)]
mod tests;
pub(crate) mod tools;

use crate::core::Core;
use crate::integrations::secrets;
use crate::model::UiEvent;
use crate::paths;
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use parking_lot::{Mutex, MutexGuard, RwLock};
use serde::Serialize;
use std::collections::HashMap;
use std::net::{Ipv4Addr, TcpListener};
use std::ops::RangeInclusive;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Weak};
use std::time::Duration;
use subtle::ConstantTimeEq;
use tauri::Runtime;
use tokio::sync::watch;

/// Where the server's port is chosen, at its first start or when the one saved is taken.
pub const PORTS: RangeInclusive<u16> = 47000..=47999;

/// The path of the MCP endpoint: `http://127.0.0.1:<port>/mcp`.
pub const PATH: &str = "/mcp";

/// How long a stop waits for the port to close: a start right after binds it again.
const STOP_WAIT: Duration = Duration::from_secs(2);

/// Who a request comes from, as its token says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Caller {
    /// Claude Code outside Escouade (a terminal, another tool), with the token its config holds.
    External,
    /// The agent of this id, with the token its process was started with.
    Agent(String),
}

/// The server as the window is told it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    /// The port it listens on; stopped, the one saved for its next start (0 before the first).
    pub port: u16,
    /// Why it does not run though the settings want it to.
    pub error: Option<String>,
}

/// The server listening: its accept loop stops when `stop` says so (or goes), then says it
/// closed the port on `closed`.
struct Running {
    port: u16,
    stop: watch::Sender<bool>,
    closed: mpsc::Receiver<()>,
}

/// The tokens a request may bring. Compared in constant time, every one of them each time: the
/// time an answer takes says nothing of how close a token came.
#[derive(Default)]
struct Tokens {
    /// Claude outside Escouade's, once read from the keychain (or made).
    external: RwLock<Option<String>>,
    /// The agents', by agent id.
    agents: RwLock<HashMap<String, String>>,
}

impl Tokens {
    fn caller(&self, presented: &str) -> Option<Caller> {
        let presented = presented.as_bytes();
        let mut found = None;
        if let Some(t) = &*self.external.read() {
            if bool::from(t.as_bytes().ct_eq(presented)) {
                found = Some(Caller::External);
            }
        }
        for (id, t) in self.agents.read().iter() {
            if bool::from(t.as_bytes().ct_eq(presented)) {
                found = Some(Caller::Agent(id.clone()));
            }
        }
        found
    }
}

/// `McpServer::sync` held: a start or a stop is done under it, so none comes between the
/// settings a sync read and what it does, nor between a stop and the port it closes.
type SyncHeld<'a> = MutexGuard<'a, ()>;

/// The MCP server of the app, held by its core.
pub struct McpServer<R: Runtime> {
    core: Weak<Core<R>>,
    /// One start or stop at a time, and one `Core::sync_mcp` from the settings it reads to the
    /// start or stop it does.
    sync: Mutex<()>,
    running: Mutex<Option<Running>>,
    /// Why its last start failed, until it starts or is no longer wanted.
    error: Mutex<Option<String>>,
    tokens: Tokens,
    pub activity: activity::Activity,
    /// What a connection may take, read at each start (tests make them small).
    limits: Mutex<http::Limits>,
}

impl<R: Runtime> McpServer<R> {
    pub fn new(core: Weak<Core<R>>) -> Self {
        Self {
            core,
            sync: Mutex::new(()),
            running: Mutex::new(None),
            error: Mutex::new(None),
            tokens: Tokens::default(),
            activity: activity::Activity::default(),
            limits: Mutex::new(http::Limits::default()),
        }
    }

    /// Listens on 127.0.0.1:`port` (another free port of `PORTS` when it is taken, or 0), and
    /// gives the port it listens on. Already listening: its port. Never once the app quits.
    // The app starts it through `Core::sync_mcp` (which holds `sync` from the settings it reads),
    // the tests directly.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn start(&self, port: u16) -> Result<u16> {
        let held = self.sync.lock();
        self.start_held(&held, port)
    }

    fn start_held(&self, _held: &SyncHeld<'_>, port: u16) -> Result<u16> {
        let mut running = self.running.lock();
        if let Some(r) = &*running {
            return Ok(r.port);
        }
        // A sync that read the settings just before the app quit: `shutdown` has stopped it, or
        // waits for this one to stop it.
        if self.quitting() {
            return Err(anyhow!("the app is stopping"));
        }
        let launched = self.launch(port);
        *self.error.lock() = launched.as_ref().err().map(|e| format!("{e:#}"));
        let port = launched.map(|r| {
            let port = r.port;
            *running = Some(r);
            port
        });
        drop(running);
        self.tell();
        port
    }

    fn quitting(&self) -> bool {
        self.core
            .upgrade()
            .is_none_or(|c| c.quitting.load(Ordering::Acquire))
    }

    fn launch(&self, port: u16) -> Result<Running> {
        let core = self
            .core
            .upgrade()
            .ok_or_else(|| anyhow!("the app is stopping"))?;
        // The token first: without it, every request would be refused.
        self.external_token()?;
        let (listener, port) = bind(port)?;
        let (stop, stopped) = watch::channel(false);
        let (closed_tx, closed) = mpsc::sync_channel(1);
        let gate = http::Gate::new(port, std::sync::Arc::downgrade(&core));
        let limits = self.limits.lock().clone();
        // On the app's own runtime: never the single-threaded one of a caller, which a stop's
        // wait would block (`wait_closed`).
        tauri::async_runtime::spawn(async move {
            http::serve(listener, gate, stopped, limits).await;
            let _ = closed_tx.send(());
        });
        log::info!("mcp: listening on 127.0.0.1:{port}");
        Ok(Running { port, stop, closed })
    }

    /// Stops listening, once the port is closed (`STOP_WAIT` at most): a client is refused from
    /// then on. The agents' tokens stay valid for its next start. Waits for a start or a sync
    /// under way.
    pub fn stop(&self) {
        let held = self.sync.lock();
        self.stop_held(&held);
    }

    fn stop_held(&self, _held: &SyncHeld<'_>) {
        let running = self.running.lock().take();
        let had_error = self.error.lock().take().is_some();
        if let Some(r) = running {
            let _ = r.stop.send(true);
            if !wait_closed(&r.closed) {
                log::warn!("mcp: the server did not say it stopped");
            }
            log::info!("mcp: stopped (port {})", r.port);
        } else if !had_error {
            return;
        }
        self.tell();
    }

    pub fn status(&self) -> McpStatus {
        let port = self.running.lock().as_ref().map(|r| r.port);
        McpStatus {
            running: port.is_some(),
            port: port
                .or_else(|| self.core.upgrade().map(|c| c.settings.read().mcp_port))
                .unwrap_or(0),
            error: self.error.lock().clone(),
        }
    }

    /// The window is told the server's status.
    fn tell(&self) {
        if let Some(core) = self.core.upgrade() {
            core.hub.emit(UiEvent::McpStatus {
                status: self.status(),
            });
        }
    }

    /// The token of Claude outside Escouade: read from the keychain once (made at the first need,
    /// `secrets::mcp_token`). Two first needs at once get the same one.
    pub fn external_token(&self) -> Result<String> {
        if let Some(t) = &*self.tokens.external.read() {
            return Ok(t.clone());
        }
        // Held from the read to the cache: a second caller waits for the token the first makes.
        let mut kept = self.tokens.external.write();
        if let Some(t) = &*kept {
            return Ok(t.clone());
        }
        let core = self
            .core
            .upgrade()
            .ok_or_else(|| anyhow!("the app is stopping"))?;
        let token =
            secrets::mcp_token(&*core.secrets, &core.data, new_token).with_context(|| {
                tr!(
                    "Le jeton du serveur MCP n’a pas pu être gardé",
                    "The MCP server’s token could not be kept"
                )
            })?;
        *kept = Some(token.clone());
        Ok(token)
    }

    /// A new token for Claude outside Escouade in place of the one it had (« Désactiver change le
    /// jeton »): the old one is refused at once, and forgotten by the keychain and the file.
    // Allowed unused until the window's switch (M5) calls it (then drop the allow).
    #[allow(dead_code)]
    pub fn renew_external_token(&self) -> Result<String> {
        let mut kept = self.tokens.external.write();
        let core = self
            .core
            .upgrade()
            .ok_or_else(|| anyhow!("the app is stopping"))?;
        let token =
            secrets::renew_mcp_token(&*core.secrets, &core.data, new_token).with_context(|| {
                tr!(
                    "Le jeton du serveur MCP n’a pas pu être gardé",
                    "The MCP server’s token could not be kept"
                )
            })?;
        *kept = Some(token.clone());
        Ok(token)
    }

    /// A new token for the agent `agent_id` (in place of the one it had), valid until
    /// `forget_agent` or the app's stop: in memory only.
    // Allowed unused until the agents' launch (M3) calls it (then drop the allow).
    #[allow(dead_code)]
    pub fn register_agent(&self, agent_id: &str) -> String {
        let token = new_token();
        self.tokens
            .agents
            .write()
            .insert(agent_id.to_string(), token.clone());
        token
    }

    /// The agent's token is refused from now on.
    // Allowed unused until the agents' stop and removal (M3) call it (then drop the allow).
    #[allow(dead_code)]
    pub fn forget_agent(&self, agent_id: &str) {
        self.tokens.agents.write().remove(agent_id);
    }

    /// Who brings `token`: None when it is no token the server gave.
    pub fn caller(&self, token: &str) -> Option<Caller> {
        self.tokens.caller(token)
    }
}

/// Waits until the accept loop says it closed the port (`STOP_WAIT` at most); false when it did
/// not say so. On a worker of a multi-threaded runtime (an async command, a tool), the wait first
/// hands the worker's other tasks to another one, the accept loop among them maybe. Elsewhere (a
/// thread of its own, a single-threaded runtime) it blocks: the loop runs on the app's runtime.
fn wait_closed(closed: &mpsc::Receiver<()>) -> bool {
    use tokio::runtime::{Handle, RuntimeFlavor};
    let wait = || closed.recv_timeout(STOP_WAIT).is_ok();
    match Handle::try_current() {
        Ok(h) if h.runtime_flavor() == RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(wait)
        }
        _ => wait(),
    }
}

/// 32 bytes from the system's random generator, in base64url (43 characters).
fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the system's random generator");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// A listener on 127.0.0.1:`port`; when that one is taken (or 0), on a free port of `PORTS`
/// (tried from a random one, so that two instances, an end-to-end test's beside the user's,
/// rarely try the same ones).
fn bind(port: u16) -> Result<(TcpListener, u16)> {
    if port != 0 {
        match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
            Ok(l) => return Ok((l, port)),
            Err(e) => log::warn!("mcp: port {port} taken ({e}), another one is chosen"),
        }
    }
    let (first, count) = (*PORTS.start(), PORTS.len() as u32);
    let from = (uuid::Uuid::new_v4().as_u128() % u128::from(count)) as u32;
    for i in 0..count {
        let p = first + ((from + i) % count) as u16;
        if p == port {
            continue;
        }
        if let Ok(l) = TcpListener::bind((Ipv4Addr::LOCALHOST, p)) {
            return Ok((l, p));
        }
    }
    Err(anyhow!(tr!(
        "Aucun port libre entre {first} et {last} pour le serveur MCP",
        "No free port between {first} and {last} for the MCP server",
        last = PORTS.end()
    )))
}

impl<R: Runtime> Core<R> {
    /// The MCP server is wanted: « Claude peut piloter Escouade », or a project whose agents may
    /// use Escouade.
    pub fn mcp_wanted(&self) -> bool {
        // One lock at a time.
        let enabled = self.settings.read().mcp_enabled;
        enabled || self.projects.read().iter().any(|p| p.agents_use_escouade)
    }

    /// Starts or stops the MCP server as the settings want it (never once the app quits). A port
    /// chosen in place of the one saved (none yet, or taken) is saved.
    pub fn sync_mcp(&self) {
        let held = self.mcp.sync.lock();
        if !self.mcp_wanted() || self.quitting.load(Ordering::Acquire) {
            self.mcp.stop_held(&held);
            return;
        }
        let saved = self.settings.read().mcp_port;
        match self.mcp.start_held(&held, saved) {
            Ok(port) if port != saved => {
                if let Err(e) = self.keep_mcp_port(port) {
                    log::error!("mcp: port {port} not saved: {e:#}");
                }
            }
            Ok(_) => {}
            Err(e) => log::error!("mcp: not started: {e:#}"),
        }
    }

    /// Saves `port` as the MCP server's.
    fn keep_mcp_port(&self, port: u16) -> Result<()> {
        let bytes = {
            let mut s = self.settings.write();
            s.mcp_port = port;
            serde_json::to_vec_pretty(&*s)?
        };
        paths::write_atomic(&self.data.settings_file(), &bytes)?;
        Ok(())
    }
}
