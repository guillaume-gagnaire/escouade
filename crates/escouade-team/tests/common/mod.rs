//! The harness of the tests: the real `escouade-team-server`, started in the test on a temporary SQLite base,
//! behind a TCP relay the test can cut (or point at another server); devices with a keychain in memory; and a
//! transport that lies when the test tells it to.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use escouade_team::client::{Call, LiveSocket, Method, Reply};
use escouade_team::crypto::invite::InviteLink;
use escouade_team::crypto::{MemberId, Purpose, Role};
use escouade_team::wire::{self, Entries, Kind};
use escouade_team::{
    ClientError, HttpTransport, MemoryKeychain, Options, Setup, Team, Transport, Vault,
};
use escouade_team_server::config::{Config, DatabaseConfig};
use escouade_team_server::Server;

pub const NOW: u64 = 1_800_000_000;

pub fn member(n: u8) -> MemberId {
    MemberId([n; 16])
}

/// A server on its own folder, served on an address of its own.
pub struct TestServer {
    pub server: Arc<Server>,
    pub addr: SocketAddr,
    pub folder: PathBuf,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TestServer {
    pub async fn start(folder: PathBuf, change: impl FnOnce(&mut Config)) -> TestServer {
        let mut config = Config::in_folder(&folder);
        config.rates.per_address.burst = 100_000.0;
        config.rates.per_address.per_second = 100_000.0;
        config.rates.per_device.burst = 100_000.0;
        config.rates.per_device.per_second = 100_000.0;
        config.rates.pairing.burst = 100_000.0;
        config.rates.pairing.per_second = 100_000.0;
        change(&mut config);
        let server = Arc::new(Server::open(config).await.expect("the server opens"));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let s = server.clone();
        tokio::spawn(async move {
            let _ = s
                .serve(listener, None, async {
                    let _ = stopped.await;
                })
                .await;
        });
        TestServer {
            server,
            addr,
            folder,
            stop: Some(stop),
        }
    }

    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A TCP relay in front of the server: the devices reach it, the test cuts its connections or points it elsewhere.
pub struct Relay {
    pub addr: SocketAddr,
    target: Arc<Mutex<SocketAddr>>,
    conns: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    accept: tokio::task::JoinHandle<()>,
}

impl Relay {
    pub async fn to(target: SocketAddr) -> Relay {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let target = Arc::new(Mutex::new(target));
        let conns: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>> = Arc::default();
        let (t, c) = (target.clone(), conns.clone());
        let accept = tokio::spawn(async move {
            while let Ok((mut inbound, _)) = listener.accept().await {
                let to = *t.lock().unwrap();
                let task = tokio::spawn(async move {
                    if let Ok(mut outbound) = tokio::net::TcpStream::connect(to).await {
                        let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
                    }
                });
                c.lock().unwrap().push(task);
            }
        });
        Relay {
            addr,
            target,
            conns,
            accept,
        }
    }

    /// Drops every connection through it (the live ones too).
    pub fn cut(&self) {
        for task in self.conns.lock().unwrap().drain(..) {
            task.abort();
        }
    }

    pub fn point_to(&self, target: SocketAddr) {
        *self.target.lock().unwrap() = target;
        self.cut();
    }

    pub fn base(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.cut();
        self.accept.abort();
    }
}

/// What the lying transport does to the team's stream.
#[derive(Default)]
pub struct Lies {
    /// Hides the team's entries after this number.
    pub hide_after: Option<u64>,
    /// Serves this payload in place of the commit of this epoch.
    pub swap_commit: Option<(u64, Vec<u8>)>,
    /// Flips a byte of the payload of the entry of this number.
    pub flip: Option<u64>,
    /// Answers a network error after sending the next commit (the device does not hear back).
    pub lose_commit_answer: bool,
    /// The lies on a project's stream (TS-5), on the reads of the project of this hex id only.
    pub project: Option<String>,
    /// Drops the entry of this number from the project's reads.
    pub project_drop: Option<u64>,
    /// Serves the payloads of these two entries swapped.
    pub project_swap: Option<(u64, u64)>,
    /// Serves the entry of this number again, after the last.
    pub project_replay: Option<u64>,
    /// Flips a byte of the payload of the entry of this number.
    pub project_flip: Option<u64>,
    /// Hides the project's entries after this number.
    pub project_hide_after: Option<u64>,
    /// Answers a network error after sending the next operation of a project.
    pub lose_op_answer: bool,
    /// Serves these payloads as entries of their numbers, beside the real ones (a forged entry, a replay: TS-13).
    pub project_inject: Vec<(u64, Vec<u8>)>,
}

/// A transport around the real one: honest until the test sets its lies; it can hold the next commit until the
/// test lets it go, and counts the refused commits.
pub struct Liar {
    pub inner: Arc<dyn Transport>,
    pub lies: Mutex<Lies>,
    /// When set, the next commit waits: `.0` is told it is there, then it waits for `.1`.
    pub gate: Mutex<
        Option<(
            tokio::sync::oneshot::Sender<()>,
            tokio::sync::oneshot::Receiver<()>,
        )>,
    >,
    pub conflicts: AtomicUsize,
    pub sign_ins: AtomicU64,
}

impl Liar {
    pub fn new(inner: Arc<dyn Transport>) -> Arc<Liar> {
        Arc::new(Liar {
            inner,
            lies: Mutex::default(),
            gate: Mutex::new(None),
            conflicts: AtomicUsize::new(0),
            sign_ins: AtomicU64::new(0),
        })
    }

    /// The next commit waits: the receiver says it is there, the sender lets it go.
    pub fn hold_next_commit(
        &self,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (there_tx, there_rx) = tokio::sync::oneshot::channel();
        let (go_tx, go_rx) = tokio::sync::oneshot::channel();
        *self.gate.lock().unwrap() = Some((there_tx, go_rx));
        (there_rx, go_tx)
    }

    fn is_commit(call: &Call) -> bool {
        call.method == Method::Post
            && call.path == "/v1/team/stream"
            && wire::decode::<wire::AppendRequest>(&call.body).is_ok_and(|r| r.kind == Kind::Commit)
    }

    fn lie(&self, reply: &mut Reply) {
        let lies = self.lies.lock().unwrap();
        if lies.hide_after.is_none() && lies.swap_commit.is_none() && lies.flip.is_none() {
            return;
        }
        let Ok(mut page) = wire::decode::<Entries>(&reply.body) else {
            return;
        };
        if let Some(n) = lies.hide_after {
            page.entries.retain(|e| e.seq <= n);
            page.more = false;
        }
        for e in &mut page.entries {
            if let Some((epoch, payload)) = &lies.swap_commit {
                if e.kind == Kind::Commit && e.epoch == Some(*epoch) {
                    e.payload = payload.clone();
                }
            }
            if lies.flip == Some(e.seq) {
                e.payload[0] ^= 1;
            }
        }
        reply.body = wire::encode(&page);
    }
}

impl Liar {
    fn lie_project(&self, path: &str, reply: &mut Reply) {
        let lies = self.lies.lock().unwrap();
        let Some(project) = &lies.project else {
            return;
        };
        if !path.contains(project.as_str()) {
            return;
        }
        let Ok(mut page) = wire::decode::<Entries>(&reply.body) else {
            return;
        };
        if let Some(n) = lies.project_hide_after {
            page.entries.retain(|e| e.seq <= n);
            page.more = false;
        }
        if let Some(n) = lies.project_drop {
            page.entries.retain(|e| e.seq != n);
        }
        if let Some((a, b)) = lies.project_swap {
            let pa = page
                .entries
                .iter()
                .find(|e| e.seq == a)
                .map(|e| e.payload.clone());
            let pb = page
                .entries
                .iter()
                .find(|e| e.seq == b)
                .map(|e| e.payload.clone());
            if let (Some(pa), Some(pb)) = (pa, pb) {
                for e in &mut page.entries {
                    if e.seq == a {
                        e.payload = pb.clone();
                    } else if e.seq == b {
                        e.payload = pa.clone();
                    }
                }
            }
        }
        if let Some(n) = lies.project_flip {
            for e in &mut page.entries {
                if e.seq == n {
                    let i = e.payload.len() / 2;
                    e.payload[i] ^= 1;
                }
            }
        }
        if !lies.project_inject.is_empty() {
            let since: u64 = path
                .split("since=")
                .nth(1)
                .and_then(|s| s.split('&').next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            for (seq, payload) in &lies.project_inject {
                if *seq > since && !page.entries.iter().any(|e| e.seq == *seq) {
                    page.entries.push(wire::Entry {
                        seq: *seq,
                        kind: Kind::Message,
                        epoch: None,
                        device: vec![0; 16],
                        at: 0,
                        payload: payload.clone(),
                    });
                }
            }
            page.entries.sort_by_key(|e| e.seq);
            if let Some(l) = path
                .split("limit=")
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
            {
                page.entries.truncate(l);
            }
        }
        if let Some(n) = lies.project_replay {
            let again = page.entries.iter().find(|e| e.seq == n).cloned();
            let last = page.entries.last().map(|e| e.seq);
            if let (Some(mut again), Some(last), false) = (again, last, page.more) {
                again.seq = last + 1;
                page.entries.push(again);
            }
        }
        reply.body = wire::encode(&page);
    }
}

#[async_trait]
impl Transport for Liar {
    async fn call(&self, call: Call) -> Result<Reply, ClientError> {
        if call.path == "/v1/auth/session" {
            self.sign_ins.fetch_add(1, Ordering::SeqCst);
        }
        let commit = Self::is_commit(&call);
        if commit {
            let gate = self.gate.lock().unwrap().take();
            if let Some((there, go)) = gate {
                let _ = there.send(());
                let _ = go.await;
            }
        }
        let team_read = call.method == Method::Get && call.path.starts_with("/v1/team/stream");
        let project_path = call.path.clone();
        let project_read = call.method == Method::Get
            && call.path.starts_with("/v1/projects/")
            && call.path.contains("/stream");
        let project_op = call.method == Method::Post
            && call.path.starts_with("/v1/projects/")
            && wire::decode::<wire::AppendRequest>(&call.body)
                .is_ok_and(|r| r.kind == Kind::Message);
        let mut reply = self.inner.call(call).await?;
        if project_op {
            let mut lies = self.lies.lock().unwrap();
            if lies.lose_op_answer {
                lies.lose_op_answer = false;
                return Err(ClientError::Net("the answer was lost".into()));
            }
        }
        if project_read && reply.status == 200 {
            self.lie_project(&project_path, &mut reply);
        }
        if commit {
            if reply.status == 409 {
                self.conflicts.fetch_add(1, Ordering::SeqCst);
            }
            let mut lies = self.lies.lock().unwrap();
            if lies.lose_commit_answer {
                lies.lose_commit_answer = false;
                return Err(ClientError::Net("the answer was lost".into()));
            }
        }
        if team_read && reply.status == 200 {
            self.lie(&mut reply);
        }
        Ok(reply)
    }

    async fn live(&self, path: &str, token: &str) -> Result<Box<dyn LiveSocket>, ClientError> {
        self.inner.live(path, token).await
    }
}

/// A device of a test: its own folder and keychain, its transport (a liar, honest by default).
pub struct Place {
    pub folder: tempfile::TempDir,
    pub keychain: Arc<MemoryKeychain>,
    pub liar: Arc<Liar>,
    pub options: Options,
}

impl Place {
    pub fn new(base: &str) -> Place {
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let transport = Arc::new(HttpTransport::new(base, http, None, None).unwrap());
        Place {
            folder: tempfile::tempdir().unwrap(),
            keychain: Arc::new(MemoryKeychain::default()),
            liar: Liar::new(transport),
            options: Options::default(),
        }
    }

    pub fn setup(&self) -> Setup {
        Setup {
            folder: self.folder.path().join("teams"),
            vault: Vault::new(self.keychain.clone(), self.folder.path().join("data")),
            transport: self.liar.clone(),
            server: "team.example.org".into(),
            options: self.options,
        }
    }
}

/// A team's world: its server (behind a relay), and its devices.
pub struct World {
    pub server: TestServer,
    pub relay: Relay,
    pub dir: tempfile::TempDir,
    /// The PostgreSQL schema of the test, dropped by [`World::finish`].
    pub schema: Option<String>,
}

/// The server's base: SQLite, or PostgreSQL when `ESCOUADE_TEST_POSTGRES_URL` is set (TS-5, as TS-1).
#[derive(Clone, Debug)]
pub enum Backend {
    Sqlite,
    Postgres(String),
}

impl Backend {
    /// PostgreSQL when its variable is set; `None` (the test is skipped) otherwise.
    pub fn postgres() -> Option<Backend> {
        match std::env::var("ESCOUADE_TEST_POSTGRES_URL") {
            Ok(url) if !url.is_empty() => Some(Backend::Postgres(url)),
            _ => {
                // The CI's Linux job has a base: there, a variant without it fails instead of passing without a word.
                assert!(
                    std::env::var("ESCOUADE_TEST_POSTGRES_REQUIRED").as_deref() != Ok("1"),
                    "ESCOUADE_TEST_POSTGRES_REQUIRED is set, ESCOUADE_TEST_POSTGRES_URL is not"
                );
                eprintln!(
                    "ESCOUADE_TEST_POSTGRES_URL is not set: the PostgreSQL variant is skipped"
                );
                None
            }
        }
    }
}

/// Each test, once on SQLite and once on PostgreSQL (skipped without its variable).
#[macro_export]
macro_rules! both {
    ($($name:ident),* $(,)?) => {
        mod sqlite {
            $(
                #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
                async fn $name() {
                    super::$name($crate::common::Backend::Sqlite).await;
                }
            )*
        }
        mod postgres {
            $(
                #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
                async fn $name() {
                    if let Some(b) = $crate::common::Backend::postgres() {
                        super::$name(b).await;
                    }
                }
            )*
        }
    };
}

impl World {
    pub async fn new() -> World {
        Self::with(|_| {}).await
    }

    pub async fn with(change: impl FnOnce(&mut Config)) -> World {
        let dir = tempfile::tempdir().unwrap();
        let server = TestServer::start(dir.path().join("server"), change).await;
        let relay = Relay::to(server.addr).await;
        World {
            server,
            relay,
            dir,
            schema: None,
        }
    }

    /// A world whose server is on `backend`.
    pub async fn on(backend: &Backend) -> World {
        Self::on_with(backend, |_| {}).await
    }

    pub async fn on_with(backend: &Backend, change: impl FnOnce(&mut Config)) -> World {
        let schema = match backend {
            Backend::Sqlite => None,
            Backend::Postgres(_) => {
                let mut b = [0u8; 8];
                getrandom::fill(&mut b).unwrap();
                Some(format!("c_{}", hex::encode(b)))
            }
        };
        let (b, s) = (backend.clone(), schema.clone());
        let mut world = Self::with(move |c| {
            if let (Backend::Postgres(url), Some(s)) = (b, s) {
                c.database = DatabaseConfig::Postgres {
                    url,
                    schema: Some(s),
                };
            }
            change(c)
        })
        .await;
        world.schema = schema;
        world
    }

    /// Stops the server, and drops the PostgreSQL schema of the test.
    pub async fn finish(mut self) {
        self.server.stop();
        if let (Some(schema), Ok(url)) = (
            self.schema.take(),
            std::env::var("ESCOUADE_TEST_POSTGRES_URL"),
        ) {
            if let Ok(pg) = escouade_team_server::db::postgres::Postgres::connect(&url, None).await
            {
                let _ = pg.drop_schema(&schema).await;
            }
        }
    }

    pub fn place(&self) -> Place {
        Place::new(&self.relay.base())
    }

    /// The owner's device creates the team.
    pub async fn create(&self, place: &Place, who: MemberId) -> Team {
        let token = self.server.server.new_team_token().await.unwrap();
        let (team, _recovery) = Team::create(place.setup(), &token, who, None, NOW)
            .await
            .expect("the team is created");
        team
    }

    /// The whole invitation: the admin's link and token, the invitee's request, the admission, the Welcome.
    pub async fn join(&self, admin: &mut Team, place: &Place, who: MemberId, role: Role) -> Team {
        let (link, token) = admin
            .invite(Purpose::NewMember(role), NOW)
            .await
            .expect("an invitation");
        // The link travels as text.
        let link = InviteLink::parse(&link.to_url()).unwrap();
        let token = token.expect("an administrator gets the server's token");
        let mut device = Team::join(place.setup(), &token, &link, who)
            .await
            .expect("the device enrols and asks");
        admin.sync().await.unwrap();
        let request = admin
            .requests()
            .last()
            .cloned()
            .expect("the admin sees the request");
        // The person who invites compares the six words with the invitee's screen (SR2-08).
        let words = device
            .join_words()
            .expect("the invitee shows its six words")
            .join(" ");
        admin.admit(&request, &words, NOW).await.expect("admitted");
        let synced = device.sync().await.unwrap();
        assert!(synced.joined, "the device joins by its Welcome");
        device
    }
}
