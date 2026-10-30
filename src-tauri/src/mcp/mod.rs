//! The MCP server (« Claude et les agents peuvent piloter Escouade »): Claude Code, in a terminal,
//! another tool or an agent of Escouade, reads the projects, agents and tickets and acts on them through the
//! tools of `tools`. On HTTP, on 127.0.0.1 only (`http`: hyper, guards of its own, then the
//! official SDK `rmcp`), each request with a bearer token: the one for Claude outside Escouade
//! (in the keychain, written into Claude Code's config) or an agent's (`grant_agent`: in memory,
//! and in the config file its process is started with, both gone when it stops). It runs while
//! the settings want it (`Core::sync_mcp`) and stops with the app.

pub(crate) mod act;
#[cfg(test)]
mod act_tests;
pub(crate) mod activity;
#[cfg(test)]
mod agents_tests;
mod background;
#[cfg(test)]
mod background_tests;
#[cfg(test)]
mod declare_tests;
mod docs;
#[cfg(test)]
mod docs_tests;
#[cfg(test)]
mod engines_tests;
mod files;
#[cfg(test)]
mod files_tests;
mod http;
mod images;
#[cfg(test)]
mod images_tests;
mod index;
#[cfg(test)]
mod index_tests;
pub(crate) mod install;
#[cfg(test)]
mod install_tests;
pub(crate) mod memory;
#[cfg(test)]
mod memory_tests;
#[cfg(test)]
mod private_tests;
mod read;
#[cfg(test)]
mod remote_tests;
#[cfg(test)]
mod requests_tests;
mod resolve;
#[cfg(test)]
mod tests;
pub(crate) mod tools;
#[cfg(test)]
mod tools_tests;
mod wait;
mod workflow;
#[cfg(test)]
mod workflow_tests;

// The one line a text is shown as (also the plan's titles).
pub(crate) use act::visible_line;

use crate::core::Core;
use crate::engine::Escouade;
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
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Weak};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use tauri::Runtime;
use tokio::sync::watch;

/// Where the server's port is chosen, at its first start or when the one saved is taken.
pub const PORTS: RangeInclusive<u16> = 47000..=47999;

/// The path of the MCP endpoint: `http://127.0.0.1:<port>/mcp`.
pub const PATH: &str = "/mcp";

/// How long a stop waits for the port to close: a start right after binds it again. Longer in
/// the tests: the accept loop runs on the app's runtime, which every test of the binary shares,
/// and on a 3-core CI runner it went unscheduled for over 2 s (the port, still open, was not
/// taken again).
const STOP_WAIT: Duration = Duration::from_secs(if cfg!(test) { 30 } else { 2 });

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
    /// One write or removal of an agent's config at a time, with its token: a process that ends
    /// never removes the file the agent's next process is about to read (`release_agent`). The
    /// refusal file too (`Core::mcp_refusal_file`).
    agent_files: Mutex<()>,
    /// One `create_agent` at a time, from counting the project's agents at work to sending the new
    /// one its message: two calls at once do not both find a place free.
    acting: tokio::sync::Mutex<()>,
    /// One declaration in Claude Code at a time (`Core::declare_now`).
    declaring: tokio::sync::Mutex<()>,
    /// « Claude et les agents peuvent piloter Escouade » was turned off since the last declaration run: that run
    /// takes the entry out of every account (the token was changed when it was turned off,
    /// `turned_off`).
    withdraw: AtomicBool,
    /// Where the server stands in each active account of each harness it is declared in, as the
    /// last run left it.
    declared: Mutex<Vec<install::Declaration>>,
    /// When the window last brought a declaration run (`Core::declare_on_focus`).
    focus_run: Mutex<Option<std::time::Instant>>,
    /// How many declaration runs are over (tests wait for the one they cause).
    #[cfg(test)]
    runs: std::sync::atomic::AtomicUsize,
    /// The harnesses the tests say are installed (a unit test finds no program of its own).
    #[cfg(test)]
    pub(crate) installed: Mutex<Vec<crate::model::EngineId>>,
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
            agent_files: Mutex::new(()),
            acting: tokio::sync::Mutex::new(()),
            declaring: tokio::sync::Mutex::new(()),
            withdraw: AtomicBool::new(false),
            declared: Mutex::new(Vec::new()),
            #[cfg(test)]
            runs: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            installed: Mutex::new(Vec::new()),
            focus_run: Mutex::new(None),
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
        if let Some(r) = &*self.running.lock() {
            return Ok(r.port);
        }
        // A sync that read the settings just before the app quit: `shutdown` has stopped it, or
        // waits for this one to stop it.
        if self.quitting() {
            return Err(anyhow!("the app is stopping"));
        }
        // `running` is not held while the port is waited for (seconds, when it is held): the
        // window reads it (`status`). Nothing else fills it, a start or a stop being under `sync`.
        let launched = self.launch(port);
        *self.error.lock() = launched.as_ref().err().map(|e| format!("{e:#}"));
        let port = launched.map(|r| {
            let port = r.port;
            *self.running.lock() = Some(r);
            port
        });
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
        let (listener, port) = bind(port, &|| self.quitting())?;
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

    /// « Claude et les agents peuvent piloter Escouade » was turned off: the token changes now, in the save itself
    /// (the old one opens nothing from this instant, though the app quits before any run, or a
    /// command of it hangs), the agents' tokens are forgotten with their files (the agents already
    /// running lose the tools at once: turned on again, they get them back by a restart only), and
    /// the next declaration run takes the entry out of every account.
    pub(crate) fn turned_off(&self) {
        if let Err(e) = self.renew_external_token() {
            log::error!("mcp: token not changed: {e:#}");
        }
        {
            let _files = self.agent_files.lock();
            self.tokens.agents.write().clear();
            self.remove_agent_configs();
        }
        self.withdraw.store(true, Ordering::Release);
    }

    /// Where the server stands in each active account of each harness it is declared in, as the
    /// last declaration left it (none while « Claude et les agents peuvent piloter Escouade » is
    /// off).
    pub fn declared(&self) -> Vec<install::Declaration> {
        self.declared.lock().clone()
    }

    /// Keeps where the server stands in the accounts, and tells the window when that changed.
    fn tell_declared(&self, declared: Vec<install::Declaration>) {
        {
            let mut kept = self.declared.lock();
            if *kept == declared {
                return;
            }
            kept.clone_from(&declared);
        }
        if let Some(core) = self.core.upgrade() {
            core.hub.emit(UiEvent::McpDeclared { declared });
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
    /// `forget_agent`, `release_agent` or the app's stop: in memory only.
    pub fn register_agent(&self, agent_id: &str) -> String {
        let token = new_token();
        self.tokens
            .agents
            .write()
            .insert(agent_id.to_string(), token.clone());
        token
    }

    /// What the agent's next process reaches the server at `port` with, as `engine` hands it: a
    /// new token (the one it had is refused from now on), in the file the engine wants written
    /// (`Engine::grant_file`: `<data>/mcp/<agent id>.json`, private like every file holding a
    /// token), or with none (the engine passes it otherwise, never on the command line).
    pub fn grant_agent(
        &self,
        agent_id: &str,
        port: u16,
        engine: &dyn crate::engine::Engine,
    ) -> Result<Escouade> {
        let core = self
            .core
            .upgrade()
            .ok_or_else(|| anyhow!("the app is stopping"))?;
        let config = core.data.mcp_agent_config(agent_id);
        let _files = self.agent_files.lock();
        let token = self.register_agent(agent_id);
        let url = format!("http://127.0.0.1:{port}{PATH}");
        let Some(body) = engine.grant_file(&url, &token) else {
            return Ok(Escouade::Granted {
                url,
                token,
                file: None,
            });
        };
        let written = std::fs::create_dir_all(core.data.mcp_agents())
            .and_then(|()| paths::write_private(&config, &body));
        if let Err(e) = written {
            self.tokens.agents.write().remove(agent_id);
            return Err(e).with_context(|| format!("{} not written", config.display()));
        }
        Ok(Escouade::Granted {
            url,
            token,
            file: Some(config),
        })
    }

    /// The agent's process given `token` stopped: the token is refused from now on and its config
    /// file goes, unless a newer process of the agent has its own already (both are then left).
    pub fn release_agent(&self, agent_id: &str, token: &str) {
        let _files = self.agent_files.lock();
        {
            let mut agents = self.tokens.agents.write();
            if agents.get(agent_id).map(String::as_str) != Some(token) {
                return;
            }
            agents.remove(agent_id);
        }
        self.remove_agent_config(agent_id);
    }

    /// The agent's token is refused from now on, and its config file goes (the agent is deleted).
    pub fn forget_agent(&self, agent_id: &str) {
        let _files = self.agent_files.lock();
        self.tokens.agents.write().remove(agent_id);
        self.remove_agent_config(agent_id);
    }

    /// `agent_files` held.
    fn remove_agent_config(&self, agent_id: &str) {
        let Some(core) = self.core.upgrade() else {
            return;
        };
        let file = core.data.mcp_agent_config(agent_id);
        match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                log::warn!("mcp: {} not removed: {e}", file.display());
            }
            _ => {}
        }
    }

    /// Every agent's config file goes: the app starts or quits, and the tokens they hold (in
    /// memory only) are none the server knows any more.
    pub fn clear_agent_configs(&self) {
        let _files = self.agent_files.lock();
        self.remove_agent_configs();
    }

    /// `agent_files` held: the folder of the agents' configs goes.
    fn remove_agent_configs(&self) {
        let Some(core) = self.core.upgrade() else {
            return;
        };
        let dir = core.data.mcp_agents();
        match std::fs::remove_dir_all(&dir) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                log::warn!("mcp: {} not removed: {e}", dir.display());
            }
            _ => {}
        }
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

/// How long the port saved for the server is waited for, when something holds it: a server that
/// stops and starts again at once (a setting turned off and on) finds its old listener not
/// quite gone, and must not move to another port (it would be declared again in every account of
/// Claude Code).
const BIND_RETRY: Duration = Duration::from_secs(3);
const BIND_STEP: Duration = Duration::from_millis(25);

/// A listener on 127.0.0.1:`port`, waited for up to `BIND_RETRY` when it is held (no longer once
/// `quitting` says the app stops); when that one stays taken (or is 0), on a free port of `PORTS`
/// (tried from a random one, so that two instances, an end-to-end test's beside the user's,
/// rarely try the same ones).
fn bind(port: u16, quitting: &dyn Fn() -> bool) -> Result<(TcpListener, u16)> {
    bind_within(port, BIND_RETRY, quitting)
}

/// A port that is busy for now, which a moment makes free again: held by a listener on its way
/// out, or, on Windows, answered « access denied » right after it was released. Any other error
/// (an address the machine does not have...) does not pass with time.
fn worth_waiting_for(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    e.kind() == ErrorKind::AddrInUse || (cfg!(windows) && e.kind() == ErrorKind::PermissionDenied)
}

/// `bind`, with at most `wait` for the port asked for (tried again every `BIND_STEP`), and none
/// once `quitting` is true: the app stops, there is no server to start.
fn bind_within(
    port: u16,
    wait: Duration,
    quitting: &dyn Fn() -> bool,
) -> Result<(TcpListener, u16)> {
    let mut refused = None;
    if port != 0 {
        let until = Instant::now() + wait;
        loop {
            match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
                Ok(l) => return Ok((l, port)),
                Err(e) if worth_waiting_for(&e) && Instant::now() < until => {
                    if quitting() {
                        return Err(anyhow!("the app is stopping"));
                    }
                    std::thread::sleep(BIND_STEP);
                }
                Err(e) => {
                    refused = Some(e);
                    break;
                }
            }
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
            if let Some(e) = refused {
                log::warn!("mcp: port {port} not available ({e}), {p} is used instead");
            }
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
    /// The MCP server is wanted: « Claude et les agents peuvent piloter Escouade », the one switch
    /// that gives it to Claude outside Escouade and to the agents of every project (ESC-91: a
    /// project no longer lets its agents use it on its own).
    pub fn mcp_wanted(&self) -> bool {
        self.settings.read().mcp_enabled
    }

    /// Starts or stops the MCP server as the settings want it (never once the app quits), then
    /// brings the declaration in Claude Code in step (`declare_in_harnesses`: a port that is another
    /// one now is declared again everywhere).
    pub fn sync_mcp(&self) {
        self.sync_server();
        self.mark_escouade_next();
        self.declare_in_harnesses();
    }

    /// Each agent whose process (or interface) runs without Escouade's server is told whether a
    /// start now would give it (« Outils d'Escouade au prochain démarrage »): the switch or the
    /// server changed. Its process is never restarted by itself, it may be at work.
    fn mark_escouade_next(&self) {
        let grants = self.escouade_granted();
        let agents: Vec<crate::agent::AgentHandle> = self.agents.read().values().cloned().collect();
        for h in agents {
            let changed = {
                let mut rt = h.lock();
                let next = grants && rt.without_escouade && (rt.proc.is_some() || rt.tui.is_some());
                std::mem::replace(&mut rt.escouade_next, next) != next
            };
            if changed {
                self.emit_agent(&h);
            }
        }
    }

    /// The server started or stopped as the settings want it. A port chosen in place of the one
    /// saved (none yet, or taken) is saved.
    fn sync_server(&self) {
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

    /// The MCP server as the app starts, before any agent does: the agents' configs a crash left
    /// go (their tokens died with it), then the server runs if the settings want it.
    pub fn start_mcp(&self) {
        self.mcp.clear_agent_configs();
        self.sync_mcp();
    }

    /// « Claude et les agents peuvent piloter Escouade » turned off: the token of Claude outside
    /// Escouade and the agents' are refused at once (`McpServer::turned_off`), and every agent
    /// whose process (or interface) runs is now without Escouade's server: turned on again, it is
    /// told it gets it at its next start (`mark_escouade_next`).
    pub(crate) fn mcp_turned_off(&self) {
        self.mcp.turned_off();
        let agents: Vec<crate::agent::AgentHandle> = self.agents.read().values().cloned().collect();
        for h in agents {
            let mut rt = h.lock();
            if rt.proc.is_some() || rt.tui.is_some() {
                rt.without_escouade = true;
            }
        }
    }

    /// A start now would give an agent Escouade's server: the switch on, the server running.
    pub(crate) fn escouade_granted(&self) -> bool {
        self.mcp_wanted() && self.mcp.status().running
    }

    /// What the agent's next process is started with of Escouade, as `engine` hands it:
    /// - the switch off, its tools refused, those of an entry of that name in the user's config of
    ///   its harness included (its removal may have failed, or the user wrote it);
    /// - on, with the server not running, the same (none to reach, and an entry of the user's
    ///   would be inherited as Claude outside Escouade's);
    /// - on, the server as the agent itself, whatever its project (ESC-91).
    ///
    /// Refused, no file is given yet: the start that wants the engine's refusal in a file of its
    /// own adds it (`with_refusal_file`), the terminal view puts it in its settings.
    pub(crate) fn agent_access(
        &self,
        agent_id: &str,
        engine: &dyn crate::engine::Engine,
    ) -> Escouade {
        let denied = Escouade::Denied { file: None };
        if !self.mcp_wanted() {
            return denied;
        }
        let status = self.mcp.status();
        if !status.running {
            return denied;
        }
        match self.mcp.grant_agent(agent_id, status.port, engine) {
            Ok(access) => access,
            Err(e) => {
                // Without a token of its own, it would reach the server through the user's entry,
                // taken for Claude outside Escouade: refused rather than taken for another.
                log::error!("mcp: agent {agent_id} not given the server: {e:#}");
                denied
            }
        }
    }

    /// `access`, refused, with the file of the engine's refusal (`Engine::refusal`), when it has
    /// one and it could be written (its tools are refused all the same, by its arguments).
    pub(crate) fn with_refusal_file(
        &self,
        agent_id: &str,
        engine: &dyn crate::engine::Engine,
        access: Escouade,
    ) -> Escouade {
        let Escouade::Denied { .. } = access else {
            return access;
        };
        let file = engine.refusal().and_then(|refusal| {
            self.mcp_refusal_file(&refusal)
                .inspect_err(|e| {
                    log::warn!("agent {agent_id}: Escouade's server not kept out: {e:#}")
                })
                .ok()
        });
        Escouade::Denied { file }
    }

    /// The file of `refusal` (`Engine::refusal`), for a process's settings: the same for every
    /// agent, written when it is not there as it should be (never while a process starting reads
    /// it, once it is).
    pub(crate) fn mcp_refusal_file(&self, refusal: &serde_json::Value) -> Result<PathBuf> {
        let file = self.data.mcp_refusal_file();
        let bytes = serde_json::to_vec_pretty(refusal)?;
        // One check and write at a time: two starts at once would share the same temporary file.
        let _files = self.mcp.agent_files.lock();
        if std::fs::read(&file).ok().as_deref() != Some(bytes.as_slice()) {
            paths::write_atomic(&file, &bytes)
                .with_context(|| format!("{} not written", file.display()))?;
        }
        Ok(file)
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
