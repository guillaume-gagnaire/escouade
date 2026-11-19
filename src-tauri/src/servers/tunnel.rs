//! The SSH tunnel to a server (ESC-537, `docs/REMOTE.md`, « Le tunnel : SSH d'abord »): the system's `ssh`, run as
//! `ssh -N -L 127.0.0.1:<local port>:127.0.0.1:<server's port> <host>`, with the user's config, agent and known hosts.
//!
//! - **Never a host key accepted in the user's place**: `BatchMode=yes` (no question, no password typed) and
//!   `StrictHostKeyChecking=yes` given on the command line, which wins over the config: an unknown or changed key ends
//!   the tunnel, and the dialog says « connecte-toi une fois par `ssh <hôte>` ». No key is read, copied or kept here.
//! - **Nothing listens but on this computer's 127.0.0.1**: the forward is bound to it in so many words; a local port
//!   taken meanwhile ends `ssh` (`ExitOnForwardFailure`), and another free one is taken.
//! - **Ready when `ssh` says so**: its `-v` lines say when the forward listens (« Local forwarding listening on … »):
//!   nothing is sent to the local port before, so nothing reaches another program that would hold it.
//! - **Watched**: what it writes is read (the last lines kept, its `debug1:` lines left out of what is shown), its end
//!   is known at once. Its failures are read into the states of
//!   `docs/SERVER-UX.md` § 3 ([`Failure`]).
//! - **Never outliving its owner** (ESC-541): `ssh` and what it starts (a `ProxyCommand`) run in a job
//!   ([`crate::job::Job`]: a Job object under Windows, a process group elsewhere), ended at once, the whole of it, when
//!   the tunnel is stopped or dropped. Under Windows the job also ends with the app however it ends (its update's
//!   `exit`, a crash): the system closes its last handle.
//! - **Under Windows**, OpenSSH of the system (`System32\OpenSSH\ssh.exe`), with no console.

use serde::Serialize;
use std::ffi::OsString;
use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The port `escouade-server` serves its API on, by default (`--port`).
pub const SERVER_PORT: u16 = 46480;

/// How long `ssh` has to reach the host (`ConnectTimeout`).
pub const CONNECT_TIMEOUT_S: u64 = 15;

/// How long the tunnel has to listen, from its start.
pub const READY_WAIT: Duration = Duration::from_secs(40);

/// The lines of `ssh` kept (the last ones).
const KEPT_LINES: usize = 60;

/// Where to reach a server: a host of `~/.ssh/config` (`acme-escouade`) or one typed (`escouade@vps.example.net`), and
/// the SSH port when typed with it (`…:2222`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dest {
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

impl Dest {
    /// A host as the user gives it: `alias`, `host`, `user@host`, `user@host:port`, `[v6]:port`. Refused: empty, a
    /// leading `-` (an option of `ssh`), spaces or control characters, a port that is not one.
    pub fn parse(typed: &str) -> Result<Dest, String> {
        let typed = typed.trim();
        let bad = |why: String| Err(why);
        if typed.is_empty() {
            return bad(tr!("Donne un hôte.", "Give a host."));
        }
        if typed.starts_with('-')
            || typed
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || "'\"`$;|&<>\\".contains(c))
        {
            return bad(tr!(
                "« {typed} » n’est pas un hôte SSH.",
                "“{typed}” is not an SSH host."
            ));
        }
        // `user@host:port`, `[v6]:port`: the port after the last `:` when what comes before is not a bare IPv6.
        let (target, port) = if let Some(end) = typed.rfind("]:") {
            let (h, p) = typed.split_at(end + 1);
            (h.to_string(), Some(&p[1..]))
        } else if typed.matches(':').count() == 1 {
            let (h, p) = typed.split_once(':').unwrap_or((typed, ""));
            (h.to_string(), Some(p))
        } else {
            (typed.to_string(), None)
        };
        let port = match port {
            None => None,
            Some(p) => match p.parse::<u16>() {
                Ok(n) if n > 0 => Some(n),
                _ => return bad(tr!("« {p} » n’est pas un port.", "“{p}” is not a port.")),
            },
        };
        let host = target.rsplit('@').next().unwrap_or("");
        if host.is_empty() || host.starts_with('-') {
            return bad(tr!(
                "« {typed} » n’est pas un hôte SSH.",
                "“{typed}” is not an SSH host."
            ));
        }
        Ok(Dest { target, port })
    }

    /// The host part (`vps.example.net` of `escouade@vps.example.net`), as a typed host says it.
    pub fn host(&self) -> &str {
        self.target.rsplit('@').next().unwrap_or(&self.target)
    }

    /// As the dialog writes it back.
    pub fn shown(&self) -> String {
        match self.port {
            Some(p) => format!("{}:{p}", self.target),
            None => self.target.clone(),
        }
    }
}

/// The `ssh` to run: the system's, or in a build of development the fake of the tests (`ESCOUADE_SSH`, with one
/// argument before the others, `ESCOUADE_SSH_PREFIX`: `node fake-ssh.mjs`).
#[derive(Debug, Clone)]
pub struct Ssh {
    pub program: PathBuf,
    pub prefix: Vec<OsString>,
    /// Variables given to it (the tests'): none for the system's.
    pub env: Vec<(OsString, OsString)>,
}

impl Ssh {
    pub fn system() -> Ssh {
        if cfg!(debug_assertions) {
            if let Some(p) = std::env::var_os("ESCOUADE_SSH").filter(|v| !v.is_empty()) {
                let prefix = std::env::var_os("ESCOUADE_SSH_PREFIX")
                    .filter(|v| !v.is_empty())
                    .into_iter()
                    .collect();
                return Ssh {
                    program: PathBuf::from(p),
                    prefix,
                    env: Vec::new(),
                };
            }
        }
        Ssh {
            program: system_ssh(),
            prefix: Vec::new(),
            env: Vec::new(),
        }
    }

    /// The command, in a process group of its own elsewhere than under Windows (for its [`Job`]).
    fn command(&self, args: &[String]) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.program);
        cmd.args(&self.prefix)
            .args(args)
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);
        cmd
    }
}

/// `child` and what it starts, ended together when this is dropped or ended ([`Job::terminate`]).
struct Job(parking_lot::Mutex<Option<crate::job::Job>>);

impl Job {
    fn of(child: &tokio::process::Child) -> Job {
        let job = crate::job::Job::for_child(child);
        if job.is_none() {
            log::warn!("servers: ssh is not in a job: it ends alone, without what it started");
        }
        Job(parking_lot::Mutex::new(job))
    }

    /// Ends the whole of it, now. Under Windows a job's end is asked, not done when the call returns: its processes
    /// are waited for (a second at most), so that none still holds what it was handed (the app exiting right after).
    fn end(&self) {
        if let Some(job) = self.0.lock().take() {
            job.terminate();
            #[cfg(windows)]
            {
                let until = std::time::Instant::now() + Duration::from_secs(1);
                while !job.pids().is_empty() && std::time::Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.end();
    }
}

/// OpenSSH of the system: under Windows, `System32\OpenSSH\ssh.exe` when it is there (not another `ssh` that a `PATH`
/// would put first, Git's); elsewhere `ssh` of the `PATH`.
fn system_ssh() -> PathBuf {
    #[cfg(windows)]
    {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let p = PathBuf::from(root)
            .join("System32")
            .join("OpenSSH")
            .join("ssh.exe");
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from("ssh")
}

/// The options that keep `ssh` from asking anything, from accepting a key, and from sharing a connection of the
/// config (`ControlMaster`): the tunnel is its own process, which ends with it.
fn safe_options(dest: &Dest) -> Vec<String> {
    let mut args: Vec<String> = [
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ControlMaster=no",
        "-o",
        "ControlPath=none",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.push("-o".into());
    args.push(format!("ConnectTimeout={CONNECT_TIMEOUT_S}"));
    if let Some(p) = dest.port {
        args.push("-p".into());
        args.push(p.to_string());
    }
    args
}

/// The arguments of the tunnel: `-N -L 127.0.0.1:<local>:127.0.0.1:<remote>`, the safe options, `-v` for its readiness.
pub fn forward_args(dest: &Dest, local: u16, remote: u16) -> Vec<String> {
    let mut args = vec![
        "-N".to_string(),
        "-v".to_string(),
        "-o".into(),
        "ExitOnForwardFailure=yes".into(),
        "-o".into(),
        "ServerAliveInterval=15".into(),
        "-o".into(),
        "ServerAliveCountMax=3".into(),
        "-L".into(),
        format!("127.0.0.1:{local}:127.0.0.1:{remote}"),
    ];
    args.extend(safe_options(dest));
    args.push(dest.target.clone());
    args
}

/// The command the user would type for the same tunnel (shown when it fails, copied to try it in a terminal).
pub fn shown_command(dest: &Dest, local: u16, remote: u16) -> String {
    let port = dest.port.map(|p| format!(" -p {p}")).unwrap_or_default();
    format!("ssh -N -L {local}:127.0.0.1:{remote}{port} {}", dest.target)
}

/// A port of 127.0.0.1 free now (the system's choice).
pub fn free_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    Ok(listener.local_addr()?.port())
}

/// Why a tunnel failed, read from what `ssh` wrote (`docs/SERVER-UX.md` § 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Why {
    /// The host is not reached: no route, refused, timed out, a name that does not resolve.
    Unreachable,
    /// The host's key is not in `known_hosts`.
    UnknownHostKey,
    /// The host's key changed since it was known.
    ChangedHostKey,
    /// The host refused the user (no key the agent offers it).
    Auth,
    /// Nothing listens on the server's port: no Escouade there, or not started.
    NoEscouade,
    /// The host refuses forwards (`AllowTcpForwarding no`).
    Forwarding,
    /// The local port was taken meanwhile: another one is taken.
    PortBusy,
    /// No `ssh` on this computer.
    NoSsh,
    /// It ended for another reason (the connection closed).
    Ended,
}

impl Why {
    /// Trying again may change something without the user doing anything.
    pub fn passing(self) -> bool {
        matches!(
            self,
            Why::Unreachable | Why::Ended | Why::PortBusy | Why::NoEscouade
        )
    }
}

/// What `ssh` wrote, read: why it failed.
pub fn classify(output: &str) -> Why {
    let o = output.to_ascii_lowercase();
    if o.contains("remote host identification has changed")
        || o.contains("host key for") && o.contains("has changed")
    {
        Why::ChangedHostKey
    } else if o.contains("host key verification failed")
        || o.contains("no matching host key")
        || o.contains("no ") && o.contains("host key is known")
    {
        Why::UnknownHostKey
    } else if o.contains("permission denied") || o.contains("too many authentication failures") {
        Why::Auth
    } else if o.contains("administratively prohibited") {
        Why::Forwarding
    } else if o.contains("open failed") || o.contains("connect failed") {
        Why::NoEscouade
    } else if o.contains("address already in use")
        || o.contains("cannot listen to port")
        || o.contains("could not request local forwarding")
    {
        Why::PortBusy
    } else if o.contains("could not resolve hostname")
        || o.contains("connection timed out")
        || o.contains("operation timed out")
        || o.contains("connection refused")
        || o.contains("network is unreachable")
        || o.contains("no route to host")
        || o.contains("name or service not known")
        || o.contains("temporary failure in name resolution")
        || o.contains("connection closed by")
        || o.contains("connection reset")
    {
        Why::Unreachable
    } else {
        Why::Ended
    }
}

/// The lines of `ssh` worth showing: its `debug1:` and the like left out.
pub fn shown_output(lines: &[String]) -> String {
    lines
        .iter()
        .filter(|l| !l.starts_with("debug") && !l.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

/// A tunnel that failed: why, what `ssh` said, the command to try by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub why: Why,
    pub output: String,
    pub command: String,
}

/// What a tunnel writes and when it ended, shared with its watcher.
#[derive(Default)]
struct Shared {
    lines: Vec<String>,
    listening: bool,
    authenticated: bool,
    ended: bool,
}

/// A running tunnel.
pub struct Tunnel {
    pub port: u16,
    pub remote: u16,
    pub dest: Dest,
    shared: Arc<parking_lot::Mutex<Shared>>,
    changed: Arc<tokio::sync::Notify>,
    stop: parking_lot::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// `ssh` and what it started.
    job: Arc<Job>,
}

impl Tunnel {
    /// `ssh` started for a tunnel of `local` (none: a free port) to the server's `remote` port of `dest`.
    pub fn start(
        ssh: &Ssh,
        dest: &Dest,
        remote: u16,
        local: Option<u16>,
    ) -> Result<Tunnel, Failure> {
        let port = match local {
            Some(p) => p,
            None => free_port().map_err(|e| Failure {
                why: Why::PortBusy,
                output: e.to_string(),
                command: String::new(),
            })?,
        };
        let command = shown_command(dest, port, remote);
        let mut cmd = ssh.command(&forward_args(dest, port, remote));
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| Failure {
            why: if e.kind() == std::io::ErrorKind::NotFound {
                Why::NoSsh
            } else {
                Why::Ended
            },
            output: e.to_string(),
            command: command.clone(),
        })?;
        let job = Arc::new(Job::of(&child));
        let shared = Arc::new(parking_lot::Mutex::new(Shared::default()));
        let changed = Arc::new(tokio::sync::Notify::new());
        // What it writes, line by line.
        if let Some(stderr) = child.stderr.take() {
            let (shared, changed) = (shared.clone(), changed.clone());
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut s = shared.lock();
                    if line.contains("Local forwarding listening on") {
                        s.listening = true;
                    }
                    if line.contains("Authenticated to")
                        || line.contains("Authentication succeeded")
                    {
                        s.authenticated = true;
                    }
                    s.lines.push(line);
                    let over = s.lines.len().saturating_sub(KEPT_LINES);
                    s.lines.drain(..over);
                    drop(s);
                    changed.notify_waiters();
                }
            });
        }
        // Its end, or the stop asked.
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let (shared, changed, job) = (shared.clone(), changed.clone(), job.clone());
            tokio::spawn(async move {
                tokio::select! {
                    _ = child.wait() => {}
                    _ = stop_rx => {
                        let _ = child.kill().await;
                    }
                }
                // What it started goes with it.
                job.end();
                // What it wrote last comes in before its end is said.
                tokio::time::sleep(Duration::from_millis(50)).await;
                shared.lock().ended = true;
                changed.notify_waiters();
            });
        }
        Ok(Tunnel {
            port,
            remote,
            dest: dest.clone(),
            shared,
            changed,
            stop: parking_lot::Mutex::new(Some(stop_tx)),
            job,
        })
    }

    /// Whether `ssh` still runs.
    pub fn alive(&self) -> bool {
        !self.shared.lock().ended
    }

    /// Why it ended (or would be said to fail now).
    pub fn failure(&self) -> Failure {
        let s = self.shared.lock();
        let all = s.lines.join("\n");
        Failure {
            why: classify(&all),
            output: shown_output(&s.lines),
            command: shown_command(&self.dest, self.port, self.remote),
        }
    }

    /// What `ssh` wrote, for a connection that failed through it (« open failed »: no Escouade there).
    pub fn output(&self) -> String {
        self.shared.lock().lines.join("\n")
    }

    /// Waits until the forward listens, as `ssh` says (or, an `ssh` that does not say it, authenticated a while ago and
    /// still there: with `ExitOnForwardFailure`, it would have ended if the forward did not listen). Nothing is
    /// connected to it: the server would count an empty connection as a failed handshake.
    pub async fn ready(&self, wait: Duration) -> Result<(), Failure> {
        let deadline = tokio::time::Instant::now() + wait;
        let mut authenticated_at: Option<tokio::time::Instant> = None;
        loop {
            let notified = self.changed.notified();
            let (listening, authenticated, ended) = {
                let s = self.shared.lock();
                (s.listening, s.authenticated, s.ended)
            };
            if ended {
                return Err(self.failure());
            }
            if authenticated && authenticated_at.is_none() {
                authenticated_at = Some(tokio::time::Instant::now());
            }
            if listening || authenticated_at.is_some_and(|t| t.elapsed() > Duration::from_secs(3)) {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                let mut f = self.failure();
                if f.why == Why::Ended {
                    f.why = Why::Unreachable;
                }
                return Err(f);
            }
            let _ = tokio::time::timeout(Duration::from_millis(150), notified).await;
        }
    }

    /// Waits until the forward listens ([`Tunnel::ready`]), then connects to it: the stream, or why it failed.
    pub async fn listening(&self, wait: Duration) -> Result<tokio::net::TcpStream, Failure> {
        let deadline = tokio::time::Instant::now() + wait;
        self.ready(wait).await?;
        loop {
            match tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, self.port)).await {
                Ok(stream) => return Ok(stream),
                Err(e) => log::debug!("servers: the tunnel's port does not answer yet: {e}"),
            }
            if !self.alive() {
                return Err(self.failure());
            }
            if tokio::time::Instant::now() >= deadline {
                let mut f = self.failure();
                f.why = Why::Unreachable;
                return Err(f);
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }

    /// Waits for its end (`ssh` gone).
    pub async fn ended(&self) {
        loop {
            let notified = self.changed.notified();
            if self.shared.lock().ended {
                return;
            }
            notified.await;
        }
    }

    /// Stops `ssh` and what it started, at once: ended when this returns, not by a task that may never run (an app
    /// that exits right after).
    pub fn stop(&self) {
        self.job.end();
        if let Some(stop) = self.stop.lock().take() {
            let _ = stop.send(());
        }
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        self.stop();
    }
}

/// `command` run on `dest` by `ssh`, with the same safe options (no question, no key accepted): its standard output,
/// or why it failed. « Le lancer d'ici par SSH » runs `escouade-server pair` this way.
pub async fn run(ssh: &Ssh, dest: &Dest, command: &str, wait: Duration) -> Result<String, Failure> {
    let mut args = safe_options(dest);
    args.push(dest.target.clone());
    args.push(command.to_string());
    let shown = format!(
        "ssh{} {} {command}",
        dest.port.map(|p| format!(" -p {p}")).unwrap_or_default(),
        dest.target
    );
    let mut cmd = ssh.command(&args);
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = cmd.spawn().map_err(|e| Failure {
        why: if e.kind() == std::io::ErrorKind::NotFound {
            Why::NoSsh
        } else {
            Why::Ended
        },
        output: e.to_string(),
        command: shown.clone(),
    })?;
    // It and what it started end with this run, answered or not.
    let _job = Job::of(&child);
    let out = match tokio::time::timeout(wait, child.wait_with_output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => {
            return Err(Failure {
                why: Why::Ended,
                output: e.to_string(),
                command: shown,
            })
        }
        Err(_) => {
            return Err(Failure {
                why: Why::Unreachable,
                output: tr!("pas de réponse à temps", "no answer in time"),
                command: shown,
            })
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    if out.status.success() {
        return Ok(stdout);
    }
    let lines: Vec<String> = stderr.lines().map(str::to_string).collect();
    let mut why = classify(&stderr);
    // The shell's « not found »: no `escouade-server` there.
    let lower = stderr.to_ascii_lowercase();
    if out.status.code() == Some(127)
        || lower.contains("not found")
        || lower.contains("introuvable")
    {
        why = Why::NoEscouade;
    }
    Err(Failure {
        why,
        output: shown_output(&lines),
        command: shown,
    })
}
