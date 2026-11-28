//! The server's own audit (AU-2, `docs/TEAM-AUDIT.md` §3.1, §4.4, §5.2): what its routes see, in opaque ids, sealed
//! by its audit key and written to a file, the standard output or syslog, by the crate `escouade-audit`.
//!
//! - **Its key**: Ed25519, drawn at `init` (and at the first `serve` of a server initialised before), kept in its data
//!   folder (`audit-key`), used to sign its events and nothing else. Its emitter's id is `server-` and 16 hexadecimal
//!   digits of the SHA-256 of its public key; `escouade-team-server audit-key` prints both, for `escouade-audit verify`
//!   and for the roster (« la clé d'audit du serveur », AU-5).
//! - **Off by default**: `--audit-file`, `--audit-stdout`, `--audit-syslog` turn it on.
//! - **One chain** for every team of the server: each event says its team (its id in hexadecimal), `server` when the
//!   server knows none (an address over its rate, a client too old, a sign-in of a device it does not know).
//! - **Never in the way**: a route hands its event to a thread of its own and goes on. The thread seals it, writes it to
//!   the buffer on the disk of each output (`<data>/audit/<output>/`), then sends it; an output that does not answer
//!   is tried again later, its buffer kept 7 days or 1 GiB, then `audit.dropped`.
//! - **No noise** (§4.3): a session once a day for a device and an address, never its renewals; a claim's taking,
//!   giving back, end and passages, never its renewals; a refusal of the same sort from the same place once a minute.

use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use ed25519_dalek::SigningKey;
use escouade_audit::event::{
    self as ev, AccessChanged, Claim as ClaimDetail, ClaimFenced, ClientTooOld, DeviceEnrolled,
    DeviceKind, DeviceRevoked, DeviceTarget, EnrollToken, Handoff, InvitationTarget, ProjectTarget,
    RateLimited, RateScope, Recipient, RoleChanged, RoleTarget, SessionRefusal, SessionRefused,
    StreamTarget, TicketTarget, TooLarge,
};
use escouade_audit::words::{Ip, Version};
use escouade_audit::{
    Actor, Anchor, Chain, Digest, Draft, Emitter, Fact, Limits, Lines, List, Nothing, Outbox,
    RotatingFile, Sink, Source, Team, Time, Token,
};
use sha2::{Digest as _, Sha256};

use crate::db::{Claim, ClaimAsk, Device, Entry, Kind, Role, Stream, TokenKind};
use crate::util::Id;

/// The file of the server's audit key, in its data folder: the 32 bytes of its seed in hexadecimal.
pub const KEY_FILE: &str = "audit-key";
/// The team of the events whose team the server does not know.
pub const NO_TEAM: &str = "server";
/// The stream of the totals of the costs is not said: every device writes there on its own rhythm (TS-16), and it is
/// their work's measure, not a security fact (§6.2).
const UNSAID_STREAMS: [Id; 1] = [crate::api::COST_TOTALS_STREAM];

/// What `--audit-ip` keeps of an address (A6-A: whole by default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IpMode {
    #[default]
    Full,
    /// IPv4 by its /24, IPv6 by its /48: the country and the operator stay, the person less.
    Truncated,
    Off,
}

impl IpMode {
    pub fn parse(s: &str) -> Result<IpMode, String> {
        match s {
            "full" => Ok(IpMode::Full),
            "truncated" => Ok(IpMode::Truncated),
            "off" => Ok(IpMode::Off),
            other => Err(format!(
                "--audit-ip takes full, truncated or off, not « {other} »"
            )),
        }
    }

    /// What goes out of `ip`.
    pub fn apply(self, ip: IpAddr) -> Option<IpAddr> {
        match (self, ip) {
            (IpMode::Off, _) => None,
            (IpMode::Full, ip) => Some(ip),
            (IpMode::Truncated, IpAddr::V4(v4)) => {
                let [a, b, c, _] = v4.octets();
                Some(IpAddr::from([a, b, c, 0]))
            }
            (IpMode::Truncated, IpAddr::V6(v6)) => {
                let s = v6.segments();
                Some(IpAddr::from([s[0], s[1], s[2], 0, 0, 0, 0, 0]))
            }
        }
    }
}

/// Where the audit goes; all off by default.
#[derive(Clone, Debug, Default)]
pub struct AuditConfig {
    /// JSON lines in this file, turned by size (100 MiB) and by day: `<dir>/<name>.jsonl`, then `<name>-<day>.<n>.jsonl`.
    pub file: Option<PathBuf>,
    /// JSON lines on the standard output (the log then goes to the standard error).
    pub stdout: bool,
    /// syslog RFC 5424, `tls://host:port` (RFC 5425) or `tcp://host:port`.
    pub syslog: Option<String>,
    /// The PEM file of the certificates syslog's TLS trusts, the system's without it.
    pub syslog_ca: Option<PathBuf>,
    /// The record in CEF in syslog's message, rather than its JSON.
    pub cef: bool,
    pub ip: IpMode,
    /// The buffer's limits (7 days, 1 GiB by default).
    pub limits: Option<Limits>,
}

impl AuditConfig {
    pub fn is_on(&self) -> bool {
        self.file.is_some() || self.stdout || self.syslog.is_some()
    }
}

/// The server's audit key in `data`, drawn and kept there when it has none. Answers it, and whether it was drawn now.
pub fn ensure_key(data: &Path) -> io::Result<(SigningKey, bool)> {
    std::fs::create_dir_all(data)?;
    let path = data.join(KEY_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let mut seed = [0u8; 32];
            hex::decode_to_slice(text.trim(), &mut seed).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: not an audit key", path.display()),
                )
            })?;
            Ok((SigningKey::from_bytes(&seed), false))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let key = Emitter::generate_key();
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt as _;
                options.mode(0o600);
            }
            let mut f = options.open(&path)?;
            use std::io::Write as _;
            f.write_all(format!("{}\n", hex::encode(key.to_bytes())).as_bytes())?;
            f.sync_all()?;
            Ok((key, true))
        }
        Err(e) => Err(e),
    }
}

/// The emitter's id of the server's key: `server-` and 16 hexadecimal digits of its public key's SHA-256.
pub fn emitter_id(key: &SigningKey) -> String {
    let digest = Sha256::digest(key.verifying_key().as_bytes());
    format!("server-{}", &hex::encode(digest)[..16])
}

/// The public key of the server's audit key (`ed25519:` and its base64).
pub fn public_key(key: &SigningKey) -> String {
    escouade_audit::words::PublicKey::of(&key.verifying_key())
        .as_str()
        .to_owned()
}

/// The keys' file `escouade-audit verify --keys` reads, for the server alone.
pub fn keys_json(key: &SigningKey) -> String {
    format!(
        "{{\"keys\": {{\"{}\": \"{}\"}}}}",
        emitter_id(key),
        public_key(key)
    )
}

fn id(bytes: &[u8]) -> escouade_audit::Id {
    escouade_audit::Id::new(hex::encode(bytes)).expect("hexadecimal is an opaque id")
}

fn team_of(team: Option<&Id>) -> Team {
    Team {
        id: match team {
            Some(t) => id(t),
            None => escouade_audit::Id::new(NO_TEAM).expect("a word"),
        },
        name: None,
    }
}

/// The name of a stream in the audit: `team`, one of the team's own streams by its name (`escouade.setting`), or a
/// project's id in hexadecimal.
pub fn stream_name(stream: Stream) -> String {
    match stream {
        Stream::Team => "team".to_owned(),
        Stream::Project(p) if crate::api::TEAM_STREAMS.contains(&p) => {
            String::from_utf8_lossy(&p).replace('/', ".")
        }
        Stream::Project(p) => hex::encode(p),
    }
}

/// The opaque id of a token in the audit: never the token nor its hash, but a digest of its hash that an
/// administrator who registered it (`token_alerts` gives it back) can make again.
pub fn token_id(hash: &[u8; 32]) -> String {
    let mut h = Sha256::new();
    h.update(b"escouade-audit/token\0");
    h.update(hash);
    format!("t-{}", &hex::encode(h.finalize())[..32])
}

fn role(r: Role) -> ev::Role {
    match r {
        Role::Admin => ev::Role::Admin,
        Role::Member => ev::Role::Member,
        Role::Service => ev::Role::Service,
    }
}

/// Who did it: a device of a team, from an address.
#[derive(Clone, Copy, Debug)]
pub struct By {
    pub team: Id,
    pub device: Id,
    pub ip: Option<IpAddr>,
}

/// The sort of a request refused (`observe` sees the status).
#[derive(Clone, Copy, Debug)]
pub enum Refusal {
    RateLimited,
    TooOld,
    TooLarge(u64),
}

/// An event handed to the thread: its team, its fact, its hour, its anchor.
struct Event {
    team: Option<Id>,
    fact: Fact,
    at: Time,
    anchor: Option<Anchor>,
}

enum Msg {
    Event(Box<Event>),
    Flush(mpsc::SyncSender<()>),
}

/// When a sort of event last went out from one place, and what came since (a rate's refusals).
struct Mark {
    last: Instant,
    window: Duration,
    pending: u64,
    rate: Option<(Option<Id>, Actor, RateScope)>,
}

/// The most places remembered for the noise (sessions, refusals): past it, the oldest windows are forgotten.
const MAX_MARKS: usize = 100_000;
const DAY: Duration = Duration::from_secs(24 * 3600);
const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3600);

/// The server's audit: off, or a thread that seals and writes.
#[derive(Default)]
pub struct Audit {
    tx: Mutex<Option<mpsc::Sender<Msg>>>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
    ip: IpMode,
    marks: Mutex<HashMap<String, Mark>>,
    emitter: Option<String>,
}

struct Output {
    name: &'static str,
    outbox: Outbox,
    sink: Box<dyn Sink + Send>,
    retry_at: Option<Instant>,
    backoff: Duration,
    trimmed_at: Option<Instant>,
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "-".to_owned())
}

impl Audit {
    /// The audit off: every event is dropped at once.
    pub fn off() -> Audit {
        Audit::default()
    }

    /// The audit of `config`, its key and its buffers in `data`; off when no output is asked.
    pub fn start(config: &AuditConfig, data: &Path) -> io::Result<Audit> {
        if !config.is_on() {
            return Ok(Audit::off());
        }
        let (key, _) = ensure_key(data)?;
        let limits = config.limits.unwrap_or_default();
        let mut outputs = Vec::new();
        let mut add = |name: &'static str, sink: Box<dyn Sink + Send>| -> io::Result<()> {
            outputs.push(Output {
                name,
                outbox: Outbox::open(data.join("audit").join(name), limits)?,
                sink,
                retry_at: None,
                backoff: Duration::from_secs(1),
                trimmed_at: None,
            });
            Ok(())
        };
        if let Some(file) = &config.file {
            let dir = file
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            let stem = file
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.strip_suffix(".jsonl").unwrap_or(n).to_owned())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "audit".to_owned());
            add(
                "file",
                Box::new(RotatingFile::new(
                    dir,
                    &stem,
                    escouade_audit::sink::DEFAULT_FILE_BYTES,
                )),
            )?;
        }
        if config.stdout {
            add("stdout", Box::new(Lines::new(io::stdout())))?;
        }
        if let Some(url) = &config.syslog {
            let target = escouade_audit::syslog::Target::parse(url)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            let roots = match &config.syslog_ca {
                Some(ca) => Some(escouade_audit::syslog::roots(Some(&std::fs::read(ca)?))?),
                None => None,
            };
            let format = escouade_audit::syslog::Format {
                hostname: hostname(),
                cef: config.cef,
                version: env!("CARGO_PKG_VERSION").to_owned(),
            };
            add(
                "syslog",
                Box::new(escouade_audit::syslog::Syslog::new(target, format, roots)?),
            )?;
        }
        // One chain for every output: it takes up at the furthest any of them kept.
        let mut chain = Chain::default();
        for o in &outputs {
            let c = o.outbox.chain()?;
            if c.seq > chain.seq {
                chain = c;
            }
        }
        let device = emitter_id(&key);
        let emitter = Emitter::new(
            key,
            escouade_audit::Id::new(device.clone()).expect("an opaque id"),
            Source::Server,
            team_of(None),
            chain,
        );
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("audit".into())
            .spawn(move || work(emitter, outputs, rx))?;
        tracing::info!(
            emitter = %device,
            file = config.file.is_some(),
            stdout = config.stdout,
            syslog = config.syslog.is_some(),
            "audit on"
        );
        Ok(Audit {
            tx: Mutex::new(Some(tx)),
            worker: Mutex::new(Some(worker)),
            ip: config.ip,
            marks: Mutex::default(),
            emitter: Some(device),
        })
    }

    pub fn is_on(&self) -> bool {
        self.tx.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// The emitter's id, when on.
    pub fn emitter(&self) -> Option<&str> {
        self.emitter.as_deref()
    }

    /// Waits until every event handed so far is written to the buffers and sent where the outputs answer.
    pub fn flush(&self) {
        let (ack, done) = mpsc::sync_channel(1);
        let sent = self
            .tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|tx| tx.send(Msg::Flush(ack)).is_ok());
        if sent {
            let _ = done.recv();
        }
    }

    /// Writes and sends what waits, then stops the thread (the server stops).
    pub fn stop(&self) {
        let tx = self.tx.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(tx);
        if let Some(worker) = self.worker.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = worker.join();
        }
    }

    fn send(&self, team: Option<&Id>, fact: Fact, at: Time, anchor: Option<Anchor>) {
        if let Some(tx) = self.tx.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = tx.send(Msg::Event(Box::new(Event {
                team: team.copied(),
                fact,
                at,
                anchor,
            })));
        }
    }

    fn now(&self, team: Option<&Id>, fact: Fact) {
        self.send(team, fact, Time::now(), None);
    }

    fn ip(&self, ip: Option<IpAddr>) -> Option<Ip> {
        ip.and_then(|ip| self.ip.apply(ip))
            .and_then(|ip| Ip::new(ip.to_string()).ok())
    }

    fn actor(&self, device: Option<&Id>, ip: Option<IpAddr>) -> Actor {
        Actor {
            device: device.map(|d| id(d)),
            ip: self.ip(ip),
            ..Default::default()
        }
    }

    /// Whether the sort `key` goes out now (none from it within `window`); counts it otherwise.
    fn once(&self, key: String, window: Duration) -> bool {
        if !self.is_on() {
            return false;
        }
        let mut marks = self.marks.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        match marks.get_mut(&key) {
            Some(m) if now.duration_since(m.last) < m.window => {
                m.pending += 1;
                false
            }
            _ => {
                if marks.len() >= MAX_MARKS {
                    marks.retain(|_, m| now.duration_since(m.last) < m.window);
                }
                marks.insert(
                    key,
                    Mark {
                        last: now,
                        window,
                        pending: 0,
                        rate: None,
                    },
                );
                true
            }
        }
    }

    /// Forgets the windows past, and says the refusals of a rate counted since its last event (every 30 s).
    pub fn sweep(&self) {
        let now = Instant::now();
        let mut said = Vec::new();
        {
            let mut marks = self.marks.lock().unwrap_or_else(|e| e.into_inner());
            marks.retain(|_, m| {
                if now.duration_since(m.last) < m.window {
                    return true;
                }
                if let (Some(rate), true) = (&m.rate, m.pending > 0) {
                    said.push((rate.clone(), m.pending));
                }
                false
            });
        }
        for ((team, actor, scope), count) in said {
            self.now(
                team.as_ref(),
                Fact::RateLimited {
                    actor,
                    target: Nothing {},
                    detail: RateLimited { scope, count },
                },
            );
        }
    }

    // === The events of the routes (§3.1) ===

    /// `/v1/enroll`: a device enrolled, by the sort of its token; the recovery token said too (`recovery.used`).
    pub fn enrolled(&self, device: &Device, token: TokenKind, ip: Option<IpAddr>) {
        let actor = self.actor(Some(&device.id), ip);
        self.now(
            Some(&device.team),
            Fact::DeviceEnrolled {
                actor: actor.clone(),
                target: DeviceTarget {
                    device: id(&device.id),
                    kind: (device.role == Role::Service).then_some(DeviceKind::Machine),
                    member: None,
                    name: None,
                },
                detail: DeviceEnrolled {
                    token: match token {
                        TokenKind::Team => EnrollToken::Init,
                        TokenKind::Device => EnrollToken::Invitation,
                        TokenKind::Recovery => EnrollToken::Recovery,
                    },
                },
            },
        );
        if token == TokenKind::Recovery {
            self.now(
                Some(&device.team),
                Fact::RecoveryUsed {
                    actor,
                    target: Nothing {},
                    detail: Nothing {},
                },
            );
        }
    }

    /// A token served a second time (an invitation link leaked): by its opaque id, never the token.
    pub fn token_reused(&self, team: Option<Id>, hash: &[u8; 32], ip: Option<IpAddr>) {
        let invitation = escouade_audit::Id::new(token_id(hash)).expect("an opaque id");
        self.now(
            team.as_ref(),
            Fact::InvitationReused {
                actor: self.actor(None, ip),
                target: InvitationTarget { invitation },
                detail: Nothing {},
            },
        );
    }

    /// A session opened: once for a device and an address, then once a day.
    pub fn session_opened(&self, device: &Device, ip: Option<IpAddr>) {
        let at = self.ip(ip).map(|i| i.to_string()).unwrap_or_default();
        if !self.once(
            format!("session.opened {} {at}", hex::encode(device.id)),
            DAY,
        ) {
            return;
        }
        self.now(
            Some(&device.team),
            Fact::SessionOpened {
                actor: self.actor(Some(&device.id), ip),
                target: Nothing {},
                detail: Nothing {},
            },
        );
    }

    /// A sign-in refused: once a minute for a device, a cause and an address.
    pub fn session_refused(
        &self,
        team: Option<Id>,
        device: &Id,
        cause: SessionRefusal,
        ip: Option<IpAddr>,
    ) {
        let at = self.ip(ip).map(|i| i.to_string()).unwrap_or_default();
        if !self.once(
            format!("session.refused {} {cause:?} {at}", hex::encode(device)),
            MINUTE,
        ) {
            return;
        }
        self.now(
            team.as_ref(),
            Fact::SessionRefused {
                actor: self.actor(Some(device), ip),
                target: Nothing {},
                detail: SessionRefused { cause },
            },
        );
    }

    /// A device revoked, by itself or an administrator.
    pub fn revoked(&self, by: By, device: &Id) {
        self.now(
            Some(&by.team),
            Fact::DeviceRevoked {
                actor: self.actor(Some(&by.device), by.ip),
                target: DeviceTarget {
                    device: id(device),
                    kind: None,
                    member: None,
                    name: None,
                },
                detail: DeviceRevoked { reason: None },
            },
        );
    }

    /// A device's role on the server changed.
    pub fn role_changed(&self, by: By, device: &Id, from: Role, to: Role) {
        if from == to {
            return;
        }
        self.now(
            Some(&by.team),
            Fact::RoleChanged {
                actor: self.actor(Some(&by.device), by.ip),
                target: RoleTarget {
                    member: None,
                    device: Some(id(device)),
                    name: None,
                },
                detail: RoleChanged {
                    from: role(from),
                    to: role(to),
                },
            },
        );
    }

    /// An entry written to a stream, with its anchor: every entry of the team's stream and of its own streams but the
    /// totals of the costs, a project's commits and voids (its operations are its work: §1.6, « hors de la liste »).
    pub fn written(&self, by: By, stream: Stream, entry: &Entry, epoch: u64) {
        let said = match stream {
            Stream::Team => true,
            Stream::Project(p) if crate::api::TEAM_STREAMS.contains(&p) => {
                !UNSAID_STREAMS.contains(&p)
            }
            Stream::Project(_) => entry.kind != Kind::Message,
        };
        if !said || !self.is_on() {
            return;
        }
        let name = Token::new(stream_name(stream)).expect("a stream's name is a word");
        self.send(
            Some(&by.team),
            Fact::StreamWritten {
                actor: self.actor(Some(&by.device), by.ip),
                target: StreamTarget {
                    stream: name.clone(),
                },
                detail: Nothing {},
            },
            Time::from_millis(entry.at.saturating_mul(1000)),
            Some(Anchor {
                stream: name,
                n: entry.seq,
                epoch,
                ct: Digest::of(&entry.payload),
            }),
        );
    }

    /// Who has access to a project: open to the team, or its devices (by 32 at most an event).
    pub fn access_changed(&self, by: By, project: &Id, everyone: bool, devices: &[Id]) {
        let chunks: Vec<&[Id]> = if devices.is_empty() {
            vec![&[]]
        } else {
            devices.chunks(ev::MAX_LIST).collect()
        };
        for chunk in chunks {
            self.now(
                Some(&by.team),
                Fact::ProjectAccessChanged {
                    actor: self.actor(Some(&by.device), by.ip),
                    target: ProjectTarget {
                        project: id(project),
                        name: None,
                    },
                    detail: AccessChanged {
                        open: everyone,
                        devices: List::new(chunk.iter().map(|d| id(d)).collect())
                            .expect("32 at most"),
                    },
                },
            );
        }
    }

    fn ticket(project: &Id, claim: &Claim) -> TicketTarget {
        TicketTarget {
            item: id(&claim.ticket),
            project: Some(id(project)),
            ticket: None,
        }
    }

    /// A claim moved: its taking (and the lease it ended, the passage it took), its giving back, its passages; never
    /// its renewals.
    pub fn claimed(
        &self,
        by: By,
        project: &Id,
        ask: ClaimAsk,
        claim: &Claim,
        before: Option<&Claim>,
    ) {
        let target = Self::ticket(project, claim);
        let actor = self.actor(Some(&by.device), by.ip);
        let team = Some(&by.team);
        match ask {
            ClaimAsk::Renew { .. } => {}
            ClaimAsk::Take { .. } => {
                if let Some(b) = before {
                    match b.holder {
                        // Another holder: its lease was over, or it was gone (revoked, without access).
                        Some(h) if h != by.device => self.now(
                            team,
                            Fact::ClaimExpired {
                                actor: self.actor(Some(&h), None),
                                target: target.clone(),
                                detail: ClaimDetail { token: b.token },
                            },
                        ),
                        // Its own, handed to it: the passage taken.
                        Some(_) if b.handed_by.is_some() => self.now(
                            team,
                            Fact::HandoffTaken {
                                actor: actor.clone(),
                                target: target.clone(),
                                detail: Handoff {
                                    to: None,
                                    branch: None,
                                },
                            },
                        ),
                        _ => {}
                    }
                }
                self.now(
                    team,
                    Fact::ClaimTaken {
                        actor,
                        target,
                        detail: ClaimDetail { token: claim.token },
                    },
                );
            }
            ClaimAsk::Release { token } => self.now(
                team,
                Fact::ClaimReleased {
                    actor,
                    target,
                    detail: ClaimDetail { token },
                },
            ),
            ClaimAsk::Hand { to, .. } => self.now(
                team,
                Fact::HandoffOffered {
                    actor,
                    target,
                    detail: Handoff {
                        to: Some(Recipient {
                            member: None,
                            device: Some(id(&to)),
                            name: None,
                        }),
                        branch: None,
                    },
                },
            ),
            ClaimAsk::Withdraw { .. } => self.now(
                team,
                Fact::HandoffWithdrawn {
                    actor,
                    target,
                    detail: Handoff {
                        to: None,
                        branch: None,
                    },
                },
            ),
        }
    }

    /// A holder's write refused under a stale token (`stale_token`).
    pub fn fenced(&self, by: By, project: &Id, ticket: &Id, token: u64, current: u64) {
        self.now(
            Some(&by.team),
            Fact::ClaimFenced {
                actor: self.actor(Some(&by.device), by.ip),
                target: TicketTarget {
                    item: id(ticket),
                    project: Some(id(project)),
                    ticket: None,
                },
                detail: ClaimFenced { token, current },
            },
        );
    }

    /// A request refused before its route: over a rate (once a minute, with the count since), from a client too old
    /// (once an hour for an address and a version), too large (once a minute for a place).
    pub fn refused(
        &self,
        refusal: Refusal,
        who: Option<(Id, Id)>,
        ip: Option<IpAddr>,
        version: Option<&str>,
    ) {
        if !self.is_on() {
            return;
        }
        let place = match who {
            Some((_, device)) => hex::encode(device),
            None => ip
                .map(|i| crate::transport::rate::address_key(i).to_string())
                .unwrap_or_default(),
        };
        let team = who.map(|(t, _)| t);
        let actor = self.actor(who.as_ref().map(|(_, d)| d), ip);
        match refusal {
            Refusal::RateLimited => {
                let scope = if who.is_some() {
                    RateScope::Device
                } else {
                    RateScope::Address
                };
                let key = format!("rate.limited {place}");
                let mut marks = self.marks.lock().unwrap_or_else(|e| e.into_inner());
                let now = Instant::now();
                let count = match marks.get_mut(&key) {
                    Some(m) if now.duration_since(m.last) < m.window => {
                        m.pending += 1;
                        return;
                    }
                    Some(m) => {
                        let count = m.pending + 1;
                        m.last = now;
                        m.pending = 0;
                        count
                    }
                    None => {
                        if marks.len() >= MAX_MARKS {
                            marks.retain(|_, m| now.duration_since(m.last) < m.window);
                        }
                        marks.insert(
                            key,
                            Mark {
                                last: now,
                                window: MINUTE,
                                pending: 0,
                                rate: Some((team, actor.clone(), scope)),
                            },
                        );
                        1
                    }
                };
                drop(marks);
                self.now(
                    team.as_ref(),
                    Fact::RateLimited {
                        actor,
                        target: Nothing {},
                        detail: RateLimited { scope, count },
                    },
                );
            }
            Refusal::TooOld => {
                let version = version
                    .and_then(|v| v.trim().parse::<u32>().ok())
                    .unwrap_or(0)
                    .to_string();
                if !self.once(format!("client.too_old {place} {version}"), HOUR) {
                    return;
                }
                self.now(
                    team.as_ref(),
                    Fact::ClientTooOld {
                        actor,
                        target: Nothing {},
                        detail: ClientTooOld {
                            version: Version::new(version).expect("a number is a version"),
                        },
                    },
                );
            }
            Refusal::TooLarge(max) => {
                if !self.once(format!("request.too_large {place} {max}"), MINUTE) {
                    return;
                }
                self.now(
                    team.as_ref(),
                    Fact::RequestTooLarge {
                        actor,
                        target: Nothing {},
                        detail: TooLarge { max },
                    },
                );
            }
        }
    }
}

impl Drop for Audit {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The thread of the audit: seals each event, keeps it in every output's buffer, sends what waits.
fn work(mut emitter: Emitter, mut outputs: Vec<Output>, rx: mpsc::Receiver<Msg>) {
    loop {
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Msg::Event(e)) => {
                let Event {
                    team,
                    fact,
                    at,
                    anchor,
                } = *e;
                seal(&mut emitter, &mut outputs, team, fact, at, anchor);
                send(&mut emitter, &mut outputs, false);
            }
            Ok(Msg::Flush(ack)) => {
                send(&mut emitter, &mut outputs, true);
                let _ = ack.send(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => send(&mut emitter, &mut outputs, false),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                send(&mut emitter, &mut outputs, true);
                return;
            }
        }
    }
}

fn seal(
    emitter: &mut Emitter,
    outputs: &mut [Output],
    team: Option<Id>,
    fact: Fact,
    at: Time,
    anchor: Option<Anchor>,
) {
    let name = fact.name();
    let sealed = match emitter.seal_for(
        team_of(team.as_ref()),
        Draft { fact, at, anchor },
        Time::now(),
    ) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(event = name, "audit: not sealed: {e}");
            return;
        }
    };
    for o in outputs.iter_mut() {
        if let Err(e) = o.outbox.push(&sealed, emitter.chain()) {
            tracing::warn!(output = o.name, "audit: the buffer: {e}");
        }
    }
}

/// Sends what waits to each output that answers (or is due again; all of them when `force`). One that does not answer
/// is tried again later (1 s, then twice as long, a minute at most), and its buffer is trimmed to its limits once a
/// minute: what it dropped is said by an `audit.dropped`, in every output.
fn send(emitter: &mut Emitter, outputs: &mut [Output], force: bool) {
    let now = Instant::now();
    let mut dropped = Vec::new();
    for o in outputs.iter_mut() {
        if !force && o.retry_at.is_some_and(|t| t > now) {
            continue;
        }
        match o.outbox.drain(o.sink.as_mut()) {
            Ok(_) => {
                if o.retry_at.take().is_some() {
                    tracing::info!(output = o.name, "audit: the output answers again");
                }
                o.backoff = Duration::from_secs(1);
            }
            Err(e) => {
                if o.retry_at.is_none() {
                    tracing::warn!(
                        output = o.name,
                        "audit: the output does not answer, its events wait on the disk: {e}"
                    );
                }
                o.retry_at = Some(now + o.backoff);
                o.backoff = (o.backoff * 2).min(MINUTE);
                if o.trimmed_at.is_none_or(|t| now.duration_since(t) >= MINUTE) {
                    o.trimmed_at = Some(now);
                    match o.outbox.trim(Time::now()) {
                        Ok(Some(d)) => dropped.push(d),
                        Ok(None) => {}
                        Err(e) => tracing::warn!(output = o.name, "audit: the buffer: {e}"),
                    }
                }
            }
        }
    }
    for d in dropped {
        tracing::warn!(
            count = d.count,
            "audit: events dropped from a buffer past its limits"
        );
        seal(
            emitter,
            outputs,
            None,
            Fact::AuditDropped {
                actor: Actor::default(),
                target: Nothing {},
                detail: d,
            },
            Time::now(),
            None,
        );
    }
}
