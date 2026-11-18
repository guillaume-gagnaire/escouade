//! « Se connecter à un serveur » (ESC-537, `docs/SERVER-UX.md` § 2 and 3, MS-3): the servers this computer knows, the
//! SSH tunnel to the one the window is on, the pairing, and the states of a connection.
//!
//! - **The servers known** (`servers.json` in the app's data, this computer's): a name (« Travail »), the SSH host
//!   (`acme-escouade`, or one typed), the server's port, the name of this computer there, the last state seen. No
//!   token: each one is kept **in the keychain of this computer, one per server** (`server-token-<id>`), or, where there
//!   is none (Linux), in `servers-tokens.json`, readable by its owner alone; never in the settings, never in the log.
//! - **The dialog** ([`Servers::attempt`]): the policy of this computer first (`servers.allowed`), then the tunnel and
//!   the handshake without a token (Escouade answers? its version?), then the pairing by a code typed or got by SSH
//!   (`escouade-server pair` run through the same host), then the handshake with the token.
//! - **The window on a server** ([`Servers::open`]): the tunnel kept up while the window is on it — `ssh` ended is
//!   started again at the delays of the window's own tries (250 ms … 5 s), on the same local port when it is still free;
//!   an end that trying again cannot mend (a host key, a refused account) stops it, and says why. The window asks where
//!   to connect ([`Servers::link`]) before each of its tries.
//! - **Forgetting a server** ([`Servers::forget`]): the server asked to revoke this computer when it answers (S15), its
//!   token and its line forgotten here; the agents of the server go on.
//!
//! A server is another machine: nothing it says is run here, its texts are data; nothing listens but on this
//! computer's 127.0.0.1.

pub mod client;
pub mod sshconfig;
pub mod tunnel;

use crate::integrations::secrets::{keep_secret, SecretStore};
use crate::model::{new_id, now_ms};
use crate::remoteapi::proto::{Machine, Revoked};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tunnel::{Dest, Failure, Ssh, Tunnel, Why};

/// The file of the servers known, in the app's data.
pub const FILE: &str = "servers.json";
/// Where the tokens go when the system has no keychain (Linux): readable by the owner alone.
pub const TOKENS_FILE: &str = "servers-tokens.json";

/// The delays between two starts of a tunnel that ended, growing to their bound (as the window's tries).
pub const RETRY_DELAYS: [u64; 6] = [250, 500, 1000, 2000, 4000, 5000];

/// How long « le lancer d'ici par SSH » waits for `escouade-server pair`.
const PAIR_OVER_SSH_WAIT: Duration = Duration::from_secs(45);

/// The command « le lancer d'ici par SSH » runs on the host: `escouade-server pair`, found on the `PATH` of a shell
/// without login, or where the installation puts it.
pub const PAIR_COMMAND: &str = "sh -c 'for p in escouade-server \"$HOME/.local/bin/escouade-server\" /usr/local/bin/escouade-server /opt/escouade/escouade-server; do if command -v \"$p\" >/dev/null 2>&1; then exec \"$p\" pair; fi; done; echo \"escouade-server: not found\" >&2; exit 127'";

/// A server this computer knows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Known {
    pub id: String,
    /// The space's name, the user's (« Travail »).
    pub name: String,
    /// The SSH host.
    pub dest: Dest,
    /// Its address (`HostName` of the config, or the host typed): what the title bar and the policy read.
    pub host: String,
    /// The port of `escouade-server` on it.
    pub remote_port: u16,
    /// This computer's name there.
    pub device: String,
    pub added_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<Machine>,
    /// When the window was last on it, connected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<i64>,
    /// What failed the last time, until it passes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<Problem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Saved {
    #[serde(default)]
    servers: Vec<Known>,
    /// The server the window is on; none: this computer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active: Option<String>,
}

/// What went wrong with a server, as `docs/SERVER-UX.md` § 3 tells it: each says what happened, and the window says
/// what did not change and what to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Problem {
    /// Refused by this computer's policy, before any connection.
    Policy { message: String },
    /// `ssh` did not get there: no route, timed out, a name unknown, an account refused, forwards refused, no `ssh`.
    #[serde(rename_all = "camelCase")]
    Unreachable {
        why: Why,
        output: String,
        command: String,
    },
    /// The host's key is not known (or changed): Escouade accepts none in the user's place.
    #[serde(rename_all = "camelCase")]
    HostKey {
        changed: bool,
        output: String,
        command: String,
    },
    /// Nothing answers as Escouade on the server's port.
    #[serde(rename_all = "camelCase")]
    NoEscouade { output: String, command: String },
    /// The pairing code refused: `wrong`, `expired`, `none`, `tries`.
    Code { reason: String, message: String },
    /// Not the same version: which side to update.
    Version {
        server: String,
        client: String,
        update: String,
    },
    /// This computer's token is no longer good there: revoked (when, by whom) or unknown.
    Revoked {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revoked: Option<Revoked>,
    },
    /// Refused by the server's own policy (`servers.remoteApi`), or for another reason it names.
    Refused { code: String, message: String },
}

impl Problem {
    fn of_failure(f: Failure) -> Problem {
        match f.why {
            Why::UnknownHostKey | Why::ChangedHostKey => Problem::HostKey {
                changed: f.why == Why::ChangedHostKey,
                output: f.output,
                command: f.command,
            },
            Why::NoEscouade => Problem::NoEscouade {
                output: f.output,
                command: f.command,
            },
            why => Problem::Unreachable {
                why,
                output: f.output,
                command: f.command,
            },
        }
    }

    fn of_refusal(r: client::Refusal, tunnel: Option<&Tunnel>, client_version: &str) -> Problem {
        match r {
            client::Refusal::NoEscouade { detail } => {
                let output = tunnel
                    .map(|t| {
                        tunnel::shown_output(
                            &t.output().lines().map(str::to_string).collect::<Vec<_>>(),
                        )
                    })
                    .filter(|o| !o.is_empty())
                    .unwrap_or(detail);
                Problem::NoEscouade {
                    output,
                    command: tunnel
                        .map(|t| tunnel::shown_command(&t.dest, t.port, t.remote))
                        .unwrap_or_default(),
                }
            }
            client::Refusal::Refused {
                code,
                error,
                server_version,
                update,
                reason,
                revoked,
            } => match code.as_str() {
                "version" => Problem::Version {
                    server: server_version.unwrap_or_default(),
                    client: client_version.to_string(),
                    update: update.unwrap_or_else(|| "server".into()),
                },
                "token" => Problem::Revoked { revoked },
                "pairing" => Problem::Code {
                    reason: reason.unwrap_or_else(|| "wrong".into()),
                    message: error,
                },
                _ => Problem::Refused {
                    code,
                    message: error,
                },
            },
        }
    }
}

/// A server as the window shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerView {
    #[serde(flatten)]
    pub known: Known,
    /// The window is on it.
    pub active: bool,
    /// The SSH host as typed back (`acme-escouade`, `escouade@vps.example.net:2222`).
    pub ssh: String,
}

/// The servers known, for « Réglages › Serveurs » and the title bar.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub servers: Vec<ServerView>,
    pub active: Option<String>,
    /// The tokens are kept in a file of this computer, not in a keychain (Linux).
    pub tokens_in_file: bool,
    /// This computer's name, as a server will be told it (« DESKTOP-7QK2 · Windows »).
    pub this_device: String,
}

/// How the tunnel of the active server stands.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum TunnelState {
    /// Being started, or started again (`tries`: how many times since it went down; `since`: when).
    #[serde(rename_all = "camelCase")]
    Starting { tries: u32, since: Option<i64> },
    /// Its forward listens.
    Up,
    /// Down for good until the user does something.
    Failed { problem: Problem },
}

/// Where the window connects to its server, before each of its tries.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub url: String,
    pub token: String,
    pub version: String,
    pub server: ServerView,
    pub tunnel: TunnelState,
}

/// The first steps of the dialog: a tunnel open to a host, before (or for) a pairing.
struct Attempt {
    tunnel: Arc<Tunnel>,
    dest: Dest,
    host: String,
    remote_port: u16,
    /// Pairing again a server known (its token revoked): the same line.
    existing: Option<String>,
}

/// What the dialog's step « Connexion » found.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reached {
    pub attempt: String,
    pub version: String,
    /// The space's name to suggest (the SSH alias, the host).
    pub name: String,
    /// This computer's name to suggest.
    pub device: String,
    pub host: String,
}

/// The answer of a step: done, or the state to show.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum Outcome<T: Serialize> {
    Ok { value: T },
    Problem { problem: Problem },
}

fn problem<T: Serialize>(p: Problem) -> Outcome<T> {
    Outcome::Problem { problem: p }
}

/// The tunnel of the server the window is on, kept up.
struct Live {
    id: String,
    port: Arc<Mutex<u16>>,
    state: Arc<Mutex<TunnelState>>,
    wake: Arc<tokio::sync::Notify>,
    stop: Arc<tokio::sync::Notify>,
}

/// The policy of this computer for a server's address (`Locks::check_ssh_server`).
pub type Policy = Arc<dyn Fn(&str) -> Result<()> + Send + Sync>;

pub struct Servers {
    file: PathBuf,
    tokens_file: PathBuf,
    store: Arc<dyn SecretStore>,
    ssh: Ssh,
    version: String,
    policy: Policy,
    ssh_config: Option<PathBuf>,
    saved: Mutex<Saved>,
    attempts: Mutex<HashMap<String, Attempt>>,
    live: Mutex<Option<Live>>,
}

fn entry(id: &str) -> String {
    format!("server-token-{id}")
}

/// « DESKTOP-7QK2 · Windows »: this computer's name, as a server is told it.
pub fn this_device() -> String {
    let machine = crate::remoteapi::this_machine();
    let os = if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    if machine.host.is_empty() {
        os.to_string()
    } else {
        format!("{} · {os}", machine.host)
    }
}

impl Servers {
    /// The servers of the data folder `root`, their tokens in `store`; `ssh` to run; `version`: the app's.
    pub fn load(
        root: &std::path::Path,
        store: Arc<dyn SecretStore>,
        ssh: Ssh,
        version: String,
        policy: Policy,
        ssh_config: Option<PathBuf>,
    ) -> Arc<Servers> {
        let file = root.join(FILE);
        let saved = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice::<Saved>(&b).ok())
            .unwrap_or_default();
        Arc::new(Servers {
            tokens_file: root.join(TOKENS_FILE),
            file,
            store,
            ssh,
            version,
            policy,
            ssh_config,
            saved: Mutex::new(saved),
            attempts: Mutex::new(HashMap::new()),
            live: Mutex::new(None),
        })
    }

    fn save(&self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&*self.saved.lock())?;
        crate::paths::write_private(&self.file, &bytes).with_context(|| format!("writing {FILE}"))
    }

    fn change(&self, f: impl FnOnce(&mut Saved)) {
        f(&mut self.saved.lock());
        if let Err(e) = self.save() {
            log::warn!("servers: {FILE} not written: {e:#}");
        }
    }

    fn known(&self, id: &str) -> Option<Known> {
        self.saved
            .lock()
            .servers
            .iter()
            .find(|k| k.id == id)
            .cloned()
    }

    fn set_problem(&self, id: &str, p: Option<Problem>) {
        self.change(|s| {
            if let Some(k) = s.servers.iter_mut().find(|k| k.id == id) {
                k.problem = p;
            }
        });
    }

    // ---------- the tokens ----------

    fn file_tokens(&self) -> BTreeMap<String, String> {
        std::fs::read(&self.tokens_file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn write_file_tokens(&self, tokens: &BTreeMap<String, String>) -> Result<()> {
        if tokens.is_empty() {
            let _ = std::fs::remove_file(&self.tokens_file);
            return Ok(());
        }
        let bytes = serde_json::to_vec_pretty(tokens)?;
        crate::paths::write_private(&self.tokens_file, &bytes)
            .with_context(|| format!("writing {TOKENS_FILE}"))
    }

    /// The token of the server `id`: the keychain's, else the file's.
    pub fn token(&self, id: &str) -> Option<String> {
        if let Ok(Some(t)) = self.store.get(&entry(id)) {
            if !t.is_empty() {
                return Some(t);
            }
        }
        self.file_tokens().remove(id).filter(|t| !t.is_empty())
    }

    /// The token of the server `id` kept: in the keychain, else (none there) in the file.
    fn keep_token(&self, id: &str, token: &str) -> Result<()> {
        match keep_secret(&*self.store, &entry(id), token) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::warn!("servers: a server's token kept in {TOKENS_FILE}, the system keychain refused it: {e:#}");
                let mut tokens = self.file_tokens();
                tokens.insert(id.to_string(), token.to_string());
                self.write_file_tokens(&tokens)
            }
        }
    }

    fn forget_token(&self, id: &str) {
        if let Err(e) = self.store.delete(&entry(id)) {
            log::warn!("servers: the system keychain did not forget a server's token: {e:#}");
        }
        let mut tokens = self.file_tokens();
        if tokens.remove(id).is_some() {
            if let Err(e) = self.write_file_tokens(&tokens) {
                log::warn!("servers: {TOKENS_FILE} not written: {e:#}");
            }
        }
    }

    // ---------- what the window reads ----------

    pub fn view(&self) -> View {
        let saved = self.saved.lock().clone();
        View {
            servers: saved
                .servers
                .iter()
                .map(|k| ServerView {
                    active: saved.active.as_deref() == Some(&k.id),
                    ssh: k.dest.shown(),
                    known: k.clone(),
                })
                .collect(),
            active: saved.active.clone(),
            tokens_in_file: !self.file_tokens().is_empty(),
            this_device: this_device(),
        }
    }

    /// The hosts of `~/.ssh/config`.
    pub fn ssh_hosts(&self) -> Vec<sshconfig::SshHost> {
        self.ssh_config
            .as_deref()
            .map(sshconfig::hosts)
            .unwrap_or_default()
    }

    /// The server the window is on.
    pub fn active(&self) -> Option<String> {
        self.saved.lock().active.clone()
    }

    /// Where the window connects to the server it is on; none on this computer.
    pub fn link(&self) -> Option<Link> {
        let id = self.active()?;
        let known = self.known(&id)?;
        let live = self.live.lock();
        let live = live.as_ref().filter(|l| l.id == id)?;
        let port = *live.port.lock();
        let tunnel = live.state.lock().clone();
        Some(Link {
            url: format!("ws://127.0.0.1:{port}{}", crate::remoteapi::proto::PATH),
            token: self.token(&id).unwrap_or_default(),
            version: self.version.clone(),
            server: ServerView {
                active: true,
                ssh: known.dest.shown(),
                known,
            },
            tunnel,
        })
    }

    // ---------- the dialog ----------

    /// The address of `dest` as the policy reads it: the config's `HostName`, else its host.
    fn host_of(&self, dest: &Dest) -> String {
        if dest.port.is_none() && !dest.target.contains('@') {
            if let Some(h) = self
                .ssh_hosts()
                .into_iter()
                .find(|h| h.alias.eq_ignore_ascii_case(&dest.target))
            {
                return h.host_name;
            }
        }
        dest.host().to_string()
    }

    /// Step « Connexion »: the policy, the tunnel to `typed`, the handshake without a token. `existing`: pairing again
    /// a server known.
    pub async fn attempt(
        &self,
        typed: &str,
        remote_port: Option<u16>,
        existing: Option<String>,
    ) -> Outcome<Reached> {
        let dest = match Dest::parse(typed) {
            Ok(d) => d,
            Err(message) => {
                return problem(Problem::Refused {
                    code: "host".into(),
                    message,
                })
            }
        };
        let host = self.host_of(&dest);
        if let Err(e) = (self.policy)(&host) {
            return problem(Problem::Policy {
                message: e.to_string(),
            });
        }
        let remote_port = remote_port.unwrap_or(tunnel::SERVER_PORT);
        let tunnel = match Tunnel::start(&self.ssh, &dest, remote_port, None) {
            Ok(t) => Arc::new(t),
            Err(f) => return problem(Problem::of_failure(f)),
        };
        let stream = match tunnel.listening(tunnel::READY_WAIT).await {
            Ok(s) => s,
            Err(f) => return problem(Problem::of_failure(f)),
        };
        let version = match client::hello(stream, tunnel.port, &self.version, "").await {
            // No token yet: refused as such, by an Escouade that says its version.
            Err(client::Refusal::Refused {
                code,
                server_version,
                ..
            }) if code == "token" => server_version.unwrap_or_default(),
            Err(r) => {
                // What `ssh` wrote of it comes in a moment after the socket closed.
                tokio::time::sleep(Duration::from_millis(300)).await;
                return problem(Problem::of_refusal(r, Some(&tunnel), &self.version));
            }
            Ok(w) => w.version,
        };
        if !version.is_empty() && version != self.version {
            return problem(Problem::Version {
                server: version.clone(),
                client: self.version.clone(),
                update: crate::remoteapi::proto::to_update(&version, &self.version).into(),
            });
        }
        let name = existing
            .as_deref()
            .and_then(|id| self.known(id))
            .map(|k| k.name)
            .unwrap_or_else(|| {
                dest.target
                    .rsplit('@')
                    .next()
                    .unwrap_or(&dest.target)
                    .to_string()
            });
        let id = new_id();
        self.attempts.lock().insert(
            id.clone(),
            Attempt {
                tunnel,
                dest,
                host: host.clone(),
                remote_port,
                existing,
            },
        );
        Outcome::Ok {
            value: Reached {
                attempt: id,
                version,
                name,
                device: this_device(),
                host,
            },
        }
    }

    fn attempt_of(&self, id: &str) -> Option<(Arc<Tunnel>, Dest)> {
        self.attempts
            .lock()
            .get(id)
            .map(|a| (a.tunnel.clone(), a.dest.clone()))
    }

    /// « Le lancer d'ici par SSH »: `escouade-server pair` run on the host of the attempt; the code it printed.
    pub async fn code_over_ssh(&self, attempt: &str) -> Outcome<String> {
        let Some((_, dest)) = self.attempt_of(attempt) else {
            return problem(gone());
        };
        match tunnel::run(&self.ssh, &dest, PAIR_COMMAND, PAIR_OVER_SSH_WAIT).await {
            Ok(out) => match find_code(&out) {
                Some(code) => Outcome::Ok { value: code },
                None => problem(Problem::NoEscouade {
                    output: out.lines().take(10).collect::<Vec<_>>().join("\n"),
                    command: format!("ssh {} escouade-server pair", dest.shown()),
                }),
            },
            Err(f) => problem(Problem::of_failure(f)),
        }
    }

    /// Step « Appairage »: `code` given to the server, this computer named `device` there; the token kept, the server
    /// known as `name`, then the handshake with the token. The id of the server.
    pub async fn pair(
        &self,
        attempt: &str,
        code: &str,
        name: &str,
        device: &str,
    ) -> Outcome<String> {
        let Some((tunnel, _)) = self.attempt_of(attempt) else {
            return problem(gone());
        };
        let connect = |t: Arc<Tunnel>| async move { t.listening(Duration::from_secs(10)).await };
        let stream = match connect(tunnel.clone()).await {
            Ok(s) => s,
            Err(f) => return problem(Problem::of_failure(f)),
        };
        let token = match client::pair(stream, tunnel.port, code, device.trim()).await {
            Ok(t) => t,
            Err(r) => return problem(Problem::of_refusal(r, Some(&tunnel), &self.version)),
        };
        let Some(a) = self.attempts.lock().remove(attempt) else {
            return problem(gone());
        };
        let id = a.existing.clone().unwrap_or_else(new_id);
        if let Err(e) = self.keep_token(&id, &token) {
            return problem(Problem::Refused {
                code: "keychain".into(),
                message: e.to_string(),
            });
        }
        // Its version and its machine, with the token.
        let welcome = match a.tunnel.listening(Duration::from_secs(10)).await {
            Ok(stream) => client::hello(stream, a.tunnel.port, &self.version, &token)
                .await
                .ok(),
            Err(_) => None,
        };
        let name = name.trim();
        let name = if name.is_empty() {
            a.dest.host().to_string()
        } else {
            name.chars().filter(|c| !c.is_control()).take(48).collect()
        };
        self.change(|s| {
            let known = Known {
                id: id.clone(),
                name: name.clone(),
                dest: a.dest.clone(),
                host: a.host.clone(),
                remote_port: a.remote_port,
                device: device.trim().to_string(),
                added_at: now_ms(),
                version: welcome.as_ref().map(|w| w.version.clone()),
                machine: welcome.as_ref().and_then(|w| w.machine.clone()),
                last_seen: None,
                problem: None,
            };
            match s.servers.iter_mut().find(|k| k.id == id) {
                Some(k) => {
                    k.problem = None;
                    k.device = known.device.clone();
                    k.version = known.version.clone();
                    k.machine = known.machine.clone();
                }
                None => s.servers.push(known),
            }
        });
        log::info!(
            "servers: this computer paired with a server ({})",
            a.dest.shown()
        );
        a.tunnel.stop();
        Outcome::Ok { value: id }
    }

    /// The dialog closed: its tunnel stopped.
    pub fn cancel(&self, attempt: &str) {
        if let Some(a) = self.attempts.lock().remove(attempt) {
            a.tunnel.stop();
        }
    }

    // ---------- the window on a server ----------

    /// The window to go on the server `id`: its tunnel up and its handshake passed with this computer's token, then it
    /// is the active one (the window loads again on it).
    pub async fn open(self: &Arc<Self>, id: &str) -> Outcome<()> {
        let Some(known) = self.known(id) else {
            return problem(gone());
        };
        if let Err(e) = (self.policy)(&known.host) {
            let p = Problem::Policy {
                message: e.to_string(),
            };
            self.set_problem(id, Some(p.clone()));
            return problem(p);
        }
        let Some(token) = self.token(id) else {
            let p = Problem::Revoked { revoked: None };
            self.set_problem(id, Some(p.clone()));
            return problem(p);
        };
        // The tunnel of the one before stopped; this one started, then kept up.
        self.leave_tunnel();
        let tunnel = match Tunnel::start(&self.ssh, &known.dest, known.remote_port, None) {
            Ok(t) => Arc::new(t),
            Err(f) => {
                let p = Problem::of_failure(f);
                self.set_problem(id, Some(p.clone()));
                return problem(p);
            }
        };
        let checked = match tunnel.listening(tunnel::READY_WAIT).await {
            Ok(stream) => client::hello(stream, tunnel.port, &self.version, &token).await,
            Err(f) => {
                let p = Problem::of_failure(f);
                self.set_problem(id, Some(p.clone()));
                return problem(p);
            }
        };
        let welcome = match checked {
            Ok(w) => w,
            Err(r) => {
                tokio::time::sleep(Duration::from_millis(300)).await;
                let p = Problem::of_refusal(r, Some(&tunnel), &self.version);
                tunnel.stop();
                self.set_problem(id, Some(p.clone()));
                return problem(p);
            }
        };
        self.change(|s| {
            if let Some(k) = s.servers.iter_mut().find(|k| k.id == id) {
                k.problem = None;
                k.version = Some(welcome.version.clone());
                k.machine = welcome.machine.clone();
                k.last_seen = Some(now_ms());
            }
            s.active = Some(id.to_string());
        });
        self.keep_up(id, tunnel);
        Outcome::Ok { value: () }
    }

    /// At the app's start, on the server it was on: its tunnel started and kept up (the window connects as it comes).
    pub fn resume(self: &Arc<Self>) {
        let Some(id) = self.active() else {
            return;
        };
        let Some(known) = self.known(&id) else {
            self.change(|s| s.active = None);
            return;
        };
        if let Err(e) = (self.policy)(&known.host) {
            log::warn!("servers: the server the window was on is refused by the policy: back on this computer");
            self.set_problem(
                &id,
                Some(Problem::Policy {
                    message: e.to_string(),
                }),
            );
            self.change(|s| s.active = None);
            return;
        }
        let port = tunnel::free_port().unwrap_or(0);
        self.supervise(id, None, port);
    }

    /// The tunnel `tunnel` (up) of the server `id` kept up from now on.
    fn keep_up(self: &Arc<Self>, id: &str, tunnel: Arc<Tunnel>) {
        let port = tunnel.port;
        self.supervise(id.to_string(), Some(tunnel), port);
    }

    fn supervise(self: &Arc<Self>, id: String, first: Option<Arc<Tunnel>>, port: u16) {
        let state = Arc::new(Mutex::new(if first.is_some() {
            TunnelState::Up
        } else {
            TunnelState::Starting {
                tries: 0,
                since: None,
            }
        }));
        let port = Arc::new(Mutex::new(port));
        let wake = Arc::new(tokio::sync::Notify::new());
        let stop = Arc::new(tokio::sync::Notify::new());
        *self.live.lock() = Some(Live {
            id: id.clone(),
            port: port.clone(),
            state: state.clone(),
            wake: wake.clone(),
            stop: stop.clone(),
        });
        let me = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            let mut current = first;
            let mut tries: u32 = 0;
            let mut since: Option<i64> = None;
            loop {
                let Some(this) = me.upgrade() else { return };
                let tunnel = match current.take() {
                    Some(t) => t,
                    None => {
                        let dest = match this.known(&id) {
                            Some(k) => (k.dest, k.remote_port),
                            None => return,
                        };
                        let wanted = *port.lock();
                        match Tunnel::start(
                            &this.ssh,
                            &dest.0,
                            dest.1,
                            Some(wanted).filter(|p| *p != 0),
                        ) {
                            Ok(t) => {
                                *port.lock() = t.port;
                                Arc::new(t)
                            }
                            Err(f) => {
                                let p = Problem::of_failure(f);
                                *state.lock() = TunnelState::Failed { problem: p.clone() };
                                this.set_problem(&id, Some(p));
                                return;
                            }
                        }
                    }
                };
                drop(this);
                // Up as it says; then until it ends, or is stopped.
                let up = tokio::select! {
                    r = tunnel.ready(tunnel::READY_WAIT) => r,
                    _ = stop.notified() => { tunnel.stop(); return; }
                };
                let failure = match up {
                    Ok(()) => {
                        if tries > 0 {
                            log::info!("servers: the tunnel is up again");
                        }
                        tries = 0;
                        since = None;
                        *state.lock() = TunnelState::Up;
                        tokio::select! {
                            _ = tunnel.ended() => {}
                            _ = stop.notified() => { tunnel.stop(); return; }
                        }
                        tunnel.failure()
                    }
                    Err(f) => f,
                };
                tunnel.stop();
                if !failure.why.passing() {
                    log::warn!("servers: the tunnel ended for good ({:?})", failure.why);
                    let p = Problem::of_failure(failure);
                    *state.lock() = TunnelState::Failed { problem: p.clone() };
                    if let Some(this) = me.upgrade() {
                        this.set_problem(&id, Some(p));
                    }
                    return;
                }
                if failure.why == Why::PortBusy {
                    // Its port taken meanwhile: another one.
                    *port.lock() = tunnel::free_port().unwrap_or(0);
                }
                since.get_or_insert_with(now_ms);
                let delay = RETRY_DELAYS[(tries as usize).min(RETRY_DELAYS.len() - 1)];
                tries += 1;
                log::info!(
                    "servers: the tunnel ended ({:?}): started again in {delay} ms (try {tries})",
                    failure.why
                );
                *state.lock() = TunnelState::Starting { tries, since };
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(delay)) => {}
                    _ = wake.notified() => {}
                    _ = stop.notified() => return,
                }
            }
        });
    }

    /// « Réessayer »: the tunnel started again now rather than at its next try; one that failed for good, tried again.
    pub fn retry(self: &Arc<Self>) {
        let failed = {
            let live = self.live.lock();
            match live.as_ref() {
                Some(l) => {
                    l.wake.notify_one();
                    matches!(*l.state.lock(), TunnelState::Failed { .. })
                        .then(|| (l.id.clone(), *l.port.lock()))
                }
                None => None,
            }
        };
        if let Some((id, port)) = failed {
            self.supervise(id, None, port);
        }
    }

    fn leave_tunnel(&self) {
        if let Some(live) = self.live.lock().take() {
            live.stop.notify_one();
            live.stop.notify_waiters();
        }
    }

    /// Back on this computer: the tunnel stopped.
    pub fn leave(&self) {
        self.leave_tunnel();
        self.change(|s| s.active = None);
    }

    /// The server `id` forgotten here (S15): asked to revoke this computer when it answers, then its token and its line
    /// forgotten. Whether the server revoked it.
    pub async fn forget(self: &Arc<Self>, id: &str) -> bool {
        let Some(known) = self.known(id) else {
            return false;
        };
        let mut revoked = false;
        if let Some(token) = self.token(id) {
            // Through the tunnel already up, else one of its own, briefly.
            let up = self
                .live
                .lock()
                .as_ref()
                .filter(|l| l.id == id && *l.state.lock() == TunnelState::Up)
                .map(|l| *l.port.lock());
            let own = if up.is_none() {
                Tunnel::start(&self.ssh, &known.dest, known.remote_port, None).ok()
            } else {
                None
            };
            let stream = match (&own, up) {
                (Some(t), _) => t.listening(Duration::from_secs(20)).await.ok(),
                (None, Some(port)) => tokio::net::TcpStream::connect(("127.0.0.1", port))
                    .await
                    .ok(),
                (None, None) => None,
            };
            let port = own.as_ref().map(|t| t.port).or(up).unwrap_or(0);
            if let Some(stream) = stream {
                revoked = client::unpair(stream, port, &token).await.is_ok();
            }
            if let Some(t) = own {
                t.stop();
            }
        }
        if self.active().as_deref() == Some(id) {
            self.leave();
        }
        self.forget_token(id);
        self.change(|s| s.servers.retain(|k| k.id != id));
        log::info!(
            "servers: a server forgotten on this computer ({}){}",
            known.dest.shown(),
            if revoked {
                ", this computer revoked there"
            } else {
                ""
            }
        );
        revoked
    }

    /// What the window saw stop its tries on the server `id` (a token revoked, another version, the policy): kept for
    /// Réglages › Serveurs, until it passes.
    pub fn note(&self, id: &str, problem: Problem) {
        self.set_problem(id, Some(problem));
    }

    /// A server's name changed.
    pub fn rename(&self, id: &str, name: &str) {
        let name: String = name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(48)
            .collect();
        if name.is_empty() {
            return;
        }
        self.change(|s| {
            if let Some(k) = s.servers.iter_mut().find(|k| k.id == id) {
                k.name = name;
            }
        });
    }

    /// The tunnels stopped: the app ends.
    pub fn shutdown(&self) {
        self.leave_tunnel();
        for (_, a) in self.attempts.lock().drain() {
            a.tunnel.stop();
        }
    }
}

fn gone() -> Problem {
    Problem::Refused {
        code: "gone".into(),
        message: tr!(
            "Cette connexion n’est plus ouverte : recommence.",
            "This connection is no longer open: start again."
        ),
    }
}

/// The pairing code in what `escouade-server pair` printed (`ABCD-EFGH`).
pub fn find_code(out: &str) -> Option<String> {
    const ALPHABET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    out.split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .find(|w| {
            w.len() == 9
                && w.as_bytes()[4] == b'-'
                && w.chars()
                    .enumerate()
                    .all(|(i, c)| i == 4 || ALPHABET.contains(c))
        })
        .map(str::to_string)
}
