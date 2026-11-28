//! What the tests share: a fake server (the protocol's server side, over an in-memory WebSocket or real TLS on
//! 127.0.0.1), a fake DNS whose answers change, echo hosts on 127.0.0.1 that stand for a company's, and the service
//! started on a temporary `connector.toml`. No network but the loopback; keys made in each test, in temporary folders.
#![allow(dead_code)]

use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use escouade_connector::guard::{Guards, Resolve};
use escouade_connector::journal::{Journal, Lines};
use escouade_connector::keys::{Pairing, State};
use escouade_connector::run::{self, Dial, Options};
use escouade_connector_proto::transport::{ws_config, ClientConfig};
use escouade_connector_proto::*;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;

pub const SERVER_V: Version = Version::new(2, 2, 10);
pub const SERVER_URL: &str = "wss://escouade.test:8443/connector";

/// A DNS whose answers the test writes, one list per call (the last repeated), and that counts its calls.
#[derive(Default)]
pub struct FakeDns {
    answers: Mutex<HashMap<String, Vec<Vec<IpAddr>>>>,
    calls: Mutex<HashMap<String, usize>>,
}

impl FakeDns {
    pub fn new() -> Arc<FakeDns> {
        Arc::new(FakeDns::default())
    }

    /// `host` answers `seq[0]` at its first call, `seq[1]` at its second…, the last one ever after.
    pub fn set(&self, host: &str, seq: &[&[&str]]) {
        let seq = seq
            .iter()
            .map(|a| a.iter().map(|s| s.parse().unwrap()).collect())
            .collect();
        self.answers.lock().unwrap().insert(host.into(), seq);
    }

    pub fn calls(&self, host: &str) -> usize {
        self.calls.lock().unwrap().get(host).copied().unwrap_or(0)
    }
}

impl Resolve for FakeDns {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        _port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<IpAddr>>> + Send + 'a>> {
        Box::pin(async move {
            let n = {
                let mut calls = self.calls.lock().unwrap();
                let n = calls.entry(host.to_string()).or_insert(0);
                *n += 1;
                *n - 1
            };
            let answers = self.answers.lock().unwrap();
            match answers.get(host) {
                Some(seq) => Ok(seq[n.min(seq.len() - 1)].clone()),
                None => Err(io::Error::new(io::ErrorKind::NotFound, "no such host")),
            }
        })
    }
}

/// A host on 127.0.0.1 that echoes what it reads, and counts its connections.
pub struct Echo {
    pub port: u16,
    pub accepted: Arc<Mutex<usize>>,
}

pub async fn echo() -> Echo {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let accepted = Arc::new(Mutex::new(0));
    let count = accepted.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut tcp, _)) = listener.accept().await else {
                return;
            };
            *count.lock().unwrap() += 1;
            tokio::spawn(async move {
                let (mut r, mut w) = tcp.split();
                let _ = tokio::io::copy(&mut r, &mut w).await;
                let _ = w.shutdown().await;
            });
        }
    });
    Echo { port, accepted }
}

/// The server's side of the channels, behind an in-memory dial.
pub struct MemServer {
    pub keys: Arc<Keypair>,
    pub channels: mpsc::UnboundedReceiver<ServerChannel>,
    pub refused: mpsc::UnboundedReceiver<AcceptError>,
}

/// The connector's dial into a [`MemServer`]: each try recorded (when, in the tokio clock), failing while `down`.
#[derive(Clone)]
pub struct MemDial {
    sockets: mpsc::UnboundedSender<WebSocketStream<DuplexStream>>,
    pub tries: Arc<Mutex<Vec<Instant>>>,
    pub down: Arc<Mutex<bool>>,
    pub urls: Arc<Mutex<Vec<String>>>,
}

impl Dial for MemDial {
    type Io = DuplexStream;
    async fn dial(
        &self,
        config: &ClientConfig,
    ) -> Result<WebSocketStream<DuplexStream>, ConnectError> {
        self.tries.lock().unwrap().push(Instant::now());
        self.urls.lock().unwrap().push(config.url.clone());
        if *self.down.lock().unwrap() {
            return Err(ConnectError::Net("connection refused (the test)".into()));
        }
        let (a, b) = tokio::io::duplex(256 << 10);
        let server = WebSocketStream::from_raw_socket(a, Role::Server, Some(ws_config())).await;
        let client = WebSocketStream::from_raw_socket(b, Role::Client, Some(ws_config())).await;
        self.sockets
            .send(server)
            .map_err(|_| ConnectError::Net("the fake server is gone".into()))?;
        Ok(client)
    }
}

/// A fake server of version `version` that pins `pinned` (a closure: the test may change its mind).
pub fn mem_server(version: Version, pinned: Arc<Mutex<Option<PublicKey>>>) -> (MemServer, MemDial) {
    let keys = Arc::new(Keypair::generate());
    let (sockets, mut incoming) = mpsc::unbounded_channel::<WebSocketStream<DuplexStream>>();
    let (tx, channels) = mpsc::unbounded_channel();
    let (refused_tx, refused) = mpsc::unbounded_channel();
    let k = keys.clone();
    tokio::spawn(async move {
        while let Some(ws) = incoming.recv().await {
            let (k, tx, refused_tx, pinned) =
                (k.clone(), tx.clone(), refused_tx.clone(), pinned.clone());
            tokio::spawn(async move {
                let known = |key: &PublicKey| *pinned.lock().unwrap() == Some(*key);
                match accept_connector(ws, &k, known, version, Limits::default()).await {
                    Ok(ch) => {
                        let _ = tx.send(ch);
                    }
                    Err(e) => {
                        let _ = refused_tx.send(e);
                    }
                }
            });
        }
    });
    (
        MemServer {
            keys,
            channels,
            refused,
        },
        MemDial {
            sockets,
            tries: Arc::default(),
            down: Arc::default(),
            urls: Arc::default(),
        },
    )
}

/// The service on a temporary `connector.toml`, paired with a [`MemServer`].
pub struct Service {
    pub dir: tempfile::TempDir,
    pub config_path: PathBuf,
    pub state: State,
    pub lines: Lines,
    pub server: MemServer,
    pub dial: MemDial,
    pub dns: Arc<FakeDns>,
    pub connector_keys_hex: String,
    pub pinned: Arc<Mutex<Option<PublicKey>>>,
    pub stop: watch::Sender<bool>,
    pub task: tokio::task::JoinHandle<Result<(), run::RunError>>,
}

pub struct Setup {
    pub toml: String,
    pub guards: Guards,
    pub server_version: Version,
    pub paired: bool,
    pub dns: Arc<FakeDns>,
}

impl Setup {
    pub fn new(toml: impl Into<String>, dns: Arc<FakeDns>) -> Setup {
        Setup {
            toml: toml.into(),
            guards: Guards {
                loopback_for_tests: true,
            },
            server_version: SERVER_V,
            paired: true,
            dns,
        }
    }
}

pub async fn service(setup: Setup) -> Service {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("connector.toml");
    std::fs::write(&config_path, &setup.toml).unwrap();
    let state = State::new(dir.path().join("state"));
    let keys = Keypair::generate();
    let pinned = Arc::new(Mutex::new(Some(keys.public())));
    let (server, dial) = mem_server(setup.server_version, pinned.clone());
    if setup.paired {
        state
            .save(
                &Pairing {
                    server: SERVER_URL.into(),
                    server_key: server.keys.public(),
                    connector_key: keys.public(),
                },
                &keys,
            )
            .unwrap();
    }
    let (journal, lines) = Journal::memory();
    let (stop, stop_rx) = watch::channel(false);
    let opts = Options {
        config_path: config_path.clone(),
        state: state.clone(),
        journal,
        resolver: setup.dns.clone(),
        guards: setup.guards,
        version: escouade_connector::version(),
        system_roots: false,
        reload_every: Duration::from_millis(50),
    };
    let task = tokio::spawn(run::run(opts, dial.clone(), stop_rx));
    Service {
        dir,
        config_path,
        state,
        lines,
        server,
        dial,
        dns: setup.dns,
        connector_keys_hex: hex::encode(keys.private_bytes()),
        pinned,
        stop,
        task,
    }
}

impl Service {
    pub async fn channel(&mut self) -> ServerChannel {
        tokio::time::timeout(Duration::from_secs(20), self.server.channels.recv())
            .await
            .expect("a channel in time")
            .expect("a channel")
    }

    pub async fn stop(self) -> Lines {
        let _ = self.stop.send(true);
        let _ = tokio::time::timeout(Duration::from_secs(20), self.task).await;
        self.lines
    }
}

/// Waits (in real time, 20 s at most) for a line of the journal that `pred` takes.
pub async fn wait_event(lines: &Lines, event: &str, pred: impl Fn(&Value) -> bool) -> Value {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(e) = lines.of(event).into_iter().find(|e| pred(e)) {
            return e;
        }
        if std::time::Instant::now() > deadline {
            panic!(
                "no event « {event} » in time; the journal:\n{}",
                lines.text()
            );
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// An open through the channel, a round trip of `payload`; the bytes come back.
pub async fn round_trip(ch: &ServerChannel, host: &str, port: u16, payload: &[u8]) -> Vec<u8> {
    let mut stream = ch.open(host, port, "agent-1", "Démo").await.unwrap();
    stream.write_all(payload).await.unwrap();
    stream.shutdown().await.unwrap();
    let mut back = Vec::new();
    stream.read_to_end(&mut back).await.unwrap();
    back
}

pub fn refused_reason(e: OpenError) -> (RefusalCode, String) {
    match e {
        OpenError::Refused(r) => (r.code, r.reason),
        other => panic!("not a refusal of the connector: {other:?}"),
    }
}
