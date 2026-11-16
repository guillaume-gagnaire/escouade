//! The harness of the integration tests: a real server on localhost, on a temporary SQLite base, or on PostgreSQL
//! when `ESCOUADE_TEST_POSTGRES_URL` is set (a schema of its own per test), and a client that speaks its CBOR.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::time::Duration;

use ed25519_dalek::{Signer, SigningKey};
use escouade_team_server::api::wire::*;
use escouade_team_server::config::{Config, DatabaseConfig, FilesConfig, S3Config};
use escouade_team_server::db::Kind;
use escouade_team_server::transport::cbor;
use escouade_team_server::util::random;
use escouade_team_server::{auth, Server, PROTOCOL};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;

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

pub struct TestServer {
    pub base: String,
    pub addr: SocketAddr,
    pub server: std::sync::Arc<Server>,
    pub dir: tempfile::TempDir,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    schema: Option<String>,
}

pub fn config(backend: &Backend, dir: &std::path::Path) -> (Config, Option<String>) {
    let mut config = Config::in_folder(dir);
    let mut schema = None;
    if let Backend::Postgres(url) = backend {
        let s = format!("t_{}", hex::encode(random::<8>()));
        config.database = DatabaseConfig::Postgres {
            url: url.clone(),
            schema: Some(s.clone()),
        };
        schema = Some(s);
    }
    // The tests do many requests from one address.
    config.rates.per_address.burst = 100_000.0;
    config.rates.per_address.per_second = 100_000.0;
    config.rates.per_device.burst = 100_000.0;
    config.rates.per_device.per_second = 100_000.0;
    (config, schema)
}

/// The S3 store of `ESCOUADE_TEST_S3_ENDPOINT` (with `_BUCKET`, `_ACCESS_KEY`, `_SECRET_KEY`), if set.
pub fn s3() -> Option<FilesConfig> {
    let endpoint = std::env::var("ESCOUADE_TEST_S3_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())?;
    Some(FilesConfig::S3(S3Config {
        endpoint,
        bucket: std::env::var("ESCOUADE_TEST_S3_BUCKET").unwrap_or_else(|_| "escouade-test".into()),
        region: std::env::var("ESCOUADE_TEST_S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
        access_key: std::env::var("ESCOUADE_TEST_S3_ACCESS_KEY").unwrap_or_default(),
        secret_key: std::env::var("ESCOUADE_TEST_S3_SECRET_KEY").unwrap_or_default(),
        prefix: format!("t{}/", hex::encode(random::<6>())),
    }))
}

impl TestServer {
    pub async fn start(backend: &Backend) -> TestServer {
        Self::start_with(backend, |_| {}).await
    }

    pub async fn start_with(backend: &Backend, change: impl FnOnce(&mut Config)) -> TestServer {
        let dir = tempfile::tempdir().unwrap();
        let (mut config, schema) = config(backend, dir.path());
        change(&mut config);
        let server = std::sync::Arc::new(Server::open(config).await.expect("the server opens"));
        Self::serve(server, dir, schema, None).await
    }

    pub async fn serve(
        server: std::sync::Arc<Server>,
        dir: tempfile::TempDir,
        schema: Option<String>,
        tls: Option<std::sync::Arc<rustls::ServerConfig>>,
    ) -> TestServer {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let s = server.clone();
        let scheme = if tls.is_some() { "https" } else { "http" };
        tokio::spawn(async move {
            s.serve(listener, tls, async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
        });
        TestServer {
            base: format!("{scheme}://{addr}"),
            addr,
            server,
            dir,
            stop: Some(stop),
            schema,
        }
    }

    pub fn client(&self) -> Client {
        Client::new(&self.base)
    }

    pub async fn team_token(&self) -> String {
        self.server.new_team_token().await.unwrap()
    }

    /// A new team, and its first device signed in.
    pub async fn new_team(&self) -> Device {
        let token = self.team_token().await;
        let mut device = Device::enroll(self, &token)
            .await
            .expect("the first device enrols");
        device.sign_in().await.unwrap();
        device
    }

    /// One more device in the admin's team, signed in.
    pub async fn add_device(&self, admin: &Device) -> Device {
        let token = admin.enrollment_token().await;
        let mut device = Device::enroll(self, &token)
            .await
            .expect("the device enrols");
        device.sign_in().await.unwrap();
        device
    }

    /// Stops the server, and drops the PostgreSQL schema of the test.
    pub async fn finish(mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let (Some(schema), Some(url)) = (
            self.schema.take(),
            std::env::var("ESCOUADE_TEST_POSTGRES_URL").ok(),
        ) {
            if let Ok(pg) = escouade_team_server::db::postgres::Postgres::connect(&url, None).await
            {
                let _ = pg.drop_schema(&schema).await;
            }
        }
    }
}

/// A refusal, as the server answers it.
#[derive(Debug, serde::Deserialize)]
pub struct Problem {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub detail: Option<ciborium::Value>,
}

#[derive(Clone)]
pub struct Client {
    pub http: reqwest::Client,
    pub base: String,
    pub protocol: Option<u32>,
    pub token: Option<String>,
}

pub enum Answer<T> {
    Ok(T),
    No(StatusCode, Problem),
}

impl<T: std::fmt::Debug> Answer<T> {
    pub fn ok(self) -> T {
        match self {
            Answer::Ok(t) => t,
            Answer::No(s, p) => panic!("refused: {s} {p:?}"),
        }
    }

    pub fn refused(self) -> (StatusCode, Problem) {
        match self {
            Answer::Ok(t) => panic!("accepted: {t:?}"),
            Answer::No(s, p) => (s, p),
        }
    }
}

impl Client {
    pub fn new(base: &str) -> Client {
        Client {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            base: base.to_string(),
            protocol: Some(PROTOCOL),
            token: None,
        }
    }

    pub fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut r = self.http.request(method, format!("{}{path}", self.base));
        if let Some(p) = self.protocol {
            r = r.header("escouade-protocol", p.to_string());
        }
        if let Some(t) = &self.token {
            r = r.bearer_auth(t);
        }
        r
    }

    pub async fn answer<R: DeserializeOwned>(r: reqwest::Response) -> Answer<R> {
        let status = r.status();
        let bytes = r.bytes().await.unwrap();
        if status.is_success() {
            Answer::Ok(cbor::decode(&bytes).expect("a CBOR answer"))
        } else {
            Answer::No(
                status,
                ciborium::from_reader(&bytes[..]).expect("a CBOR problem"),
            )
        }
    }

    pub async fn post<B: Serialize, R: DeserializeOwned>(&self, path: &str, body: &B) -> Answer<R> {
        let r = self
            .request(reqwest::Method::POST, path)
            .header("content-type", "application/cbor")
            .body(cbor::encode(body))
            .send()
            .await
            .unwrap();
        Self::answer(r).await
    }

    pub async fn put<B: Serialize, R: DeserializeOwned>(&self, path: &str, body: &B) -> Answer<R> {
        let r = self
            .request(reqwest::Method::PUT, path)
            .header("content-type", "application/cbor")
            .body(cbor::encode(body))
            .send()
            .await
            .unwrap();
        Self::answer(r).await
    }

    pub async fn get<R: DeserializeOwned>(&self, path: &str) -> Answer<R> {
        let r = self
            .request(reqwest::Method::GET, path)
            .send()
            .await
            .unwrap();
        Self::answer(r).await
    }

    /// A status only (204, a refusal…).
    pub async fn status(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> (StatusCode, Vec<u8>) {
        let mut r = self.request(method, path);
        if let Some(b) = body {
            r = r.header("content-type", "application/octet-stream").body(b);
        }
        let r = r.send().await.unwrap();
        (r.status(), r.bytes().await.unwrap().to_vec())
    }
}

pub struct Device {
    pub key: SigningKey,
    pub id: Vec<u8>,
    pub team: Vec<u8>,
    pub client: Client,
}

impl Device {
    pub async fn enroll(server: &TestServer, token: &str) -> Result<Device, (StatusCode, Problem)> {
        Self::enroll_on(server.client(), token).await
    }

    /// Enrols through a given client (TLS, a server run by the binary).
    pub async fn enroll_on(client: Client, token: &str) -> Result<Device, (StatusCode, Problem)> {
        let key = SigningKey::from_bytes(&random::<32>());
        let answer: Answer<EnrollResponse> = client
            .post(
                "/v1/enroll",
                &EnrollRequest {
                    token: token.to_string(),
                    public_key: key.verifying_key().to_bytes().to_vec(),
                },
            )
            .await;
        match answer {
            Answer::Ok(r) => Ok(Device {
                key,
                id: r.device,
                team: r.team,
                client,
            }),
            Answer::No(s, p) => Err((s, p)),
        }
    }

    pub fn id_hex(&self) -> String {
        hex::encode(&self.id)
    }

    pub async fn challenge(&self) -> Answer<ChallengeResponse> {
        self.client
            .post(
                "/v1/auth/challenge",
                &ChallengeRequest {
                    device: self.id.clone(),
                },
            )
            .await
    }

    pub fn sign(&self, challenge: &[u8]) -> Vec<u8> {
        let challenge: [u8; 32] = challenge.try_into().unwrap();
        let id: [u8; 16] = self.id.as_slice().try_into().unwrap();
        self.key
            .sign(&auth::signed_message(&id, &challenge))
            .to_bytes()
            .to_vec()
    }

    pub async fn session(&self, challenge: &[u8], signature: Vec<u8>) -> Answer<TokenResponse> {
        self.client
            .post(
                "/v1/auth/session",
                &SessionRequest {
                    device: self.id.clone(),
                    challenge: challenge.to_vec(),
                    signature,
                },
            )
            .await
    }

    pub async fn sign_in(&mut self) -> Result<(), (StatusCode, Problem)> {
        let challenge = match self.challenge().await {
            Answer::Ok(c) => c.challenge,
            Answer::No(s, p) => return Err((s, p)),
        };
        let signature = self.sign(&challenge);
        match self.session(&challenge, signature).await {
            Answer::Ok(t) => {
                self.client.token = Some(t.token);
                Ok(())
            }
            Answer::No(s, p) => Err((s, p)),
        }
    }

    pub async fn enrollment_token(&self) -> String {
        let r: TokenResponse = self
            .client
            .post("/v1/devices/enrollments", &EnrollmentRequest::default())
            .await
            .ok();
        r.token
    }

    pub async fn append_team(
        &self,
        kind: Kind,
        epoch: Option<u64>,
        payload: &[u8],
    ) -> Answer<Appended> {
        self.client
            .post(
                "/v1/team/stream",
                &AppendRequest {
                    fence: None,
                    kind,
                    epoch,
                    payload: payload.to_vec(),
                },
            )
            .await
    }

    pub async fn message(&self, payload: &[u8]) -> u64 {
        self.append_team(Kind::Message, None, payload)
            .await
            .ok()
            .seq
    }

    pub async fn commit(&self, epoch: u64, payload: &[u8]) -> Answer<Appended> {
        self.append_team(Kind::Commit, Some(epoch), payload).await
    }

    pub async fn append_project(&self, project: &[u8; 16], payload: &[u8]) -> Answer<Appended> {
        self.client
            .post(
                &format!("/v1/projects/{}/stream", hex::encode(project)),
                &AppendRequest {
                    fence: None,
                    kind: Kind::Message,
                    epoch: None,
                    payload: payload.to_vec(),
                },
            )
            .await
    }

    pub async fn read_team(&self, since: u64) -> Entries {
        self.client
            .get(&format!("/v1/team/stream?since={since}"))
            .await
            .ok()
    }

    pub async fn read_project(&self, project: &[u8; 16], since: u64) -> Entries {
        self.client
            .get(&format!(
                "/v1/projects/{}/stream?since={since}",
                hex::encode(project)
            ))
            .await
            .ok()
    }

    pub async fn revoke(&self, device: &Device) -> StatusCode {
        self.client
            .status(
                reqwest::Method::POST,
                &format!("/v1/devices/{}/revoke", device.id_hex()),
                None,
            )
            .await
            .0
    }

    pub async fn put_file(&self, file: &[u8; 16], bytes: Vec<u8>) -> StatusCode {
        self.client
            .status(
                reqwest::Method::PUT,
                &format!("/v1/files/{}", hex::encode(file)),
                Some(bytes),
            )
            .await
            .0
    }

    pub async fn get_file(&self, file: &[u8; 16]) -> (StatusCode, Vec<u8>) {
        self.client
            .status(
                reqwest::Method::GET,
                &format!("/v1/files/{}", hex::encode(file)),
                None,
            )
            .await
    }

    pub async fn delete_file(&self, file: &[u8; 16]) -> StatusCode {
        self.client
            .status(
                reqwest::Method::DELETE,
                &format!("/v1/files/{}", hex::encode(file)),
                None,
            )
            .await
            .0
    }

    /// A live connection.
    pub async fn live(&self, server: &TestServer) -> Live {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut request = format!("ws://{}/v1/live", server.addr)
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("escouade-protocol", PROTOCOL.to_string().parse().unwrap());
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {}", self.client.token.as_ref().unwrap())
                .parse()
                .unwrap(),
        );
        let (socket, _) = tokio_tungstenite::connect_async(request)
            .await
            .expect("the live connection opens");
        Live { socket }
    }
}

pub struct Live {
    pub socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

#[derive(Debug, serde::Deserialize)]
pub struct Frame {
    pub stream: String,
    #[serde(default, with = "serde_bytes")]
    pub project: Option<Vec<u8>>,
    #[serde(default)]
    pub entry: Option<escouade_team_server::db::Entry>,
    #[serde(default)]
    pub claim: Option<escouade_team_server::db::Claim>,
}

#[allow(clippy::large_enum_variant)]
pub enum LiveEvent {
    Frame(Frame),
    Closed(Option<u16>),
}

impl Live {
    pub async fn next(&mut self, wait: Duration) -> Option<LiveEvent> {
        use futures_util::StreamExt as _;
        use tokio_tungstenite::tungstenite::Message;
        loop {
            let msg = tokio::time::timeout(wait, self.socket.next()).await.ok()?;
            match msg {
                Some(Ok(Message::Binary(b))) => {
                    return Some(LiveEvent::Frame(cbor::decode(&b).unwrap()))
                }
                Some(Ok(Message::Close(frame))) => {
                    return Some(LiveEvent::Closed(frame.map(|f| f.code.into())))
                }
                Some(Ok(_)) => continue,
                Some(Err(_)) | None => return Some(LiveEvent::Closed(None)),
            }
        }
    }

    pub async fn frame(&mut self) -> Frame {
        match self.next(Duration::from_secs(5)).await {
            Some(LiveEvent::Frame(f)) => f,
            Some(LiveEvent::Closed(c)) => panic!("closed: {c:?}"),
            None => panic!("no frame within 5 s"),
        }
    }
}
