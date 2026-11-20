//! The client of the team server: enrolment by token, the signed challenge and its session (renewed by itself), the
//! streams and their reading again from `since`, the encrypted files, the live connection (`/v1/live`) that comes
//! back by itself.
//!
//! Every byte goes through a [`Transport`]. The app's is [`HttpTransport`]: the core's reqwest client (its proxy, its
//! authorities, `escouade_plugins::tls::app_builder`) for the requests, and for the live connection the same proxy
//! (an HTTP CONNECT tunnel, opened by hand as the app's live transcription does) and the same TLS configuration
//! (`tls::config`). The tests put a transport of their own around it to play a server that lies.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use ed25519_dalek::{Signer as _, SigningKey};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex};

use crate::wire::{self, *};

/// What the server's signed challenge is made of (`escouade_team_server::auth::signed_message`).
const AUTH_LABEL: &[u8] = b"escouade-team-server/auth/v1\0";

/// A session is renewed this long before it ends.
const RENEW_BEFORE: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("the team server cannot be reached: {0}")]
    Net(String),
    #[error("Mets Escouade à jour : le serveur d'équipe ne parle plus cette version")]
    ClientTooOld,
    #[error("le serveur d'équipe est en retard sur cette version d'Escouade : son administrateur doit le mettre à jour")]
    ServerTooOld,
    #[error("the server refused the sign-in: this device is unknown to it, or revoked")]
    SignInRefused,
    /// Another commit holds the epoch: process it first.
    #[error("a commit already holds epoch {}", .0.expected_epoch.saturating_sub(1))]
    EpochTaken(EpochTaken),
    /// A claim refused (TS-6): `taken` (another device holds the ticket), `stale_version` (it changed since),
    /// `claim_lost` (no longer this device's), `stale_token` (a holder's write under a token taken over); the claim as
    /// it is.
    #[error("the claim is refused ({code})")]
    Claim { code: String, claim: Box<Claim> },
    #[error("the server refused ({status} {error}): {message}")]
    Refused {
        status: u16,
        error: String,
        message: String,
    },
    #[error("the server's answer is not understood: {0}")]
    Decode(String),
}

impl ClientError {
    /// The stable code of a refusal (`token_used`, `pair_tried`…).
    pub fn code(&self) -> Option<&str> {
        match self {
            ClientError::Refused { error, .. } => Some(error),
            _ => None,
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            ClientError::Refused { status, .. } => Some(*status),
            _ => None,
        }
    }
}

/// A request, as a transport sends it.
#[derive(Clone, Debug)]
pub struct Call {
    pub method: Method,
    /// From the server's root: `/v1/team/stream?since=3`.
    pub path: String,
    pub content_type: Option<&'static str>,
    pub body: Vec<u8>,
    pub token: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

/// An answer.
#[derive(Clone, Debug)]
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
}

/// What a live connection gives.
#[derive(Debug)]
pub enum LiveMessage {
    Binary(Vec<u8>),
    /// The server closed it, with this code (4001 the session ended, 4003 the device is revoked).
    Closed(Option<u16>),
}

#[async_trait]
pub trait LiveSocket: Send {
    /// The next message; None once the connection is gone.
    async fn next(&mut self) -> Option<Result<LiveMessage, ClientError>>;
}

/// How the client reaches the server.
#[async_trait]
pub trait Transport: Send + Sync {
    async fn call(&self, call: Call) -> Result<Reply, ClientError>;
    /// Opens the live connection of `path` with the session `token`.
    async fn live(&self, path: &str, token: &str) -> Result<Box<dyn LiveSocket>, ClientError>;
}

// === The app's transport ===

/// The largest answer of the server the client reads: a page of a stream (8 MiB on the server by default), a file or
/// a snapshot (25 MiB), with room for their CBOR. Past it, the answer is refused before it is read whole (its announced
/// length, or as it comes): a server that lies, or one in between, never makes the device allocate without a bound.
pub const MAX_REPLY: usize = 32 << 20;

/// The largest message of the live connection: a frame carries an entry (1 MiB on the server by default) and its
/// header. tungstenite's own bound is 64 MiB.
pub const MAX_LIVE_MESSAGE: usize = 4 << 20;

/// The body of an answer, `max` bytes at most: refused at once when its announced length is past it, else read
/// piece by piece and refused as soon as it goes past.
async fn read_bounded(mut reply: reqwest::Response, max: usize) -> Result<Vec<u8>, ClientError> {
    let too_large = || ClientError::Net(format!("the server's answer is larger than {max} bytes"));
    if reply.content_length().is_some_and(|n| n > max as u64) {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(piece) = reply
        .chunk()
        .await
        .map_err(|e| ClientError::Net(e.to_string()))?
    {
        if body.len() + piece.len() > max {
            return Err(too_large());
        }
        body.extend_from_slice(&piece);
    }
    Ok(body)
}

/// The app's transport: the core's reqwest client, and its proxy and TLS for the live connection.
pub struct HttpTransport {
    base: reqwest::Url,
    http: reqwest::Client,
    tls: Option<Arc<rustls::ClientConfig>>,
    proxy: Option<reqwest::Url>,
}

impl HttpTransport {
    /// `base`, the server's address (`https://team.example.org`); `http`, the core's client
    /// (`tls::app_builder(settings)`); `tls`, the core's TLS configuration (`tls::config(&net)`); `proxy`, the proxy
    /// the server's host goes through (`tls::proxy_for(&net, host)`), for the live connection.
    pub fn new(
        base: &str,
        http: reqwest::Client,
        tls: Option<Arc<rustls::ClientConfig>>,
        proxy: Option<reqwest::Url>,
    ) -> Result<Self, ClientError> {
        let base = reqwest::Url::parse(base).map_err(|e| ClientError::Net(e.to_string()))?;
        if !matches!(base.scheme(), "http" | "https") {
            return Err(ClientError::Net(format!(
                "{}: not an http(s) address",
                base.scheme()
            )));
        }
        Ok(Self {
            base,
            http,
            tls,
            proxy,
        })
    }

    fn url(&self, path: &str) -> Result<reqwest::Url, ClientError> {
        self.base
            .join(path)
            .map_err(|e| ClientError::Net(e.to_string()))
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn call(&self, call: Call) -> Result<Reply, ClientError> {
        let method = match call.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Delete => reqwest::Method::DELETE,
        };
        let mut r = self
            .http
            .request(method, self.url(&call.path)?)
            .header("escouade-protocol", PROTOCOL.to_string());
        if let Some(t) = &call.token {
            r = r.bearer_auth(t);
        }
        if let Some(ct) = call.content_type {
            r = r.header("content-type", ct).body(call.body);
        }
        let reply = r
            .send()
            .await
            .map_err(|e| ClientError::Net(e.to_string()))?;
        let status = reply.status().as_u16();
        let body = read_bounded(reply, MAX_REPLY).await?;
        Ok(Reply { status, body })
    }

    async fn live(&self, path: &str, token: &str) -> Result<Box<dyn LiveSocket>, ClientError> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut url = self.url(path)?;
        let secure = url.scheme() == "https";
        url.set_scheme(if secure { "wss" } else { "ws" })
            .map_err(|_| ClientError::Net("the live address".into()))?;
        let host = url
            .host_str()
            .ok_or_else(|| ClientError::Net("no host".into()))?
            .to_string();
        let port = url
            .port_or_known_default()
            .ok_or_else(|| ClientError::Net("no port".into()))?;
        let net = |e: std::io::Error| ClientError::Net(e.to_string());

        let tcp = match &self.proxy {
            Some(proxy) => tunnel(proxy, &host, port).await?,
            None => tokio::net::TcpStream::connect((host.as_str(), port))
                .await
                .map_err(net)?,
        };
        let io: Box<dyn Io> = if secure {
            let config = self.tls.clone().ok_or_else(|| {
                ClientError::Net("no TLS configuration for the live connection".into())
            })?;
            let name = rustls_pki_types::ServerName::try_from(host.clone())
                .map_err(|e| ClientError::Net(e.to_string()))?;
            let tls = tokio_rustls::TlsConnector::from(config)
                .connect(name, tcp)
                .await
                .map_err(net)?;
            Box::new(tls)
        } else {
            Box::new(tcp)
        };
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|e| ClientError::Net(e.to_string()))?;
        let headers = request.headers_mut();
        headers.insert(
            "authorization",
            format!("Bearer {token}")
                .parse()
                .map_err(|_| ClientError::Net("the token".into()))?,
        );
        headers.insert(
            "escouade-protocol",
            PROTOCOL
                .to_string()
                .parse()
                .expect("a number is a header value"),
        );
        let config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
            .max_message_size(Some(MAX_LIVE_MESSAGE))
            .max_frame_size(Some(MAX_LIVE_MESSAGE));
        let (socket, _) = tokio_tungstenite::client_async_with_config(request, io, Some(config))
            .await
            .map_err(|e| match e {
                tokio_tungstenite::tungstenite::Error::Http(r) => ClientError::Refused {
                    status: r.status().as_u16(),
                    error: "live".into(),
                    message: "the live connection was refused".into(),
                },
                e => ClientError::Net(e.to_string()),
            })?;
        Ok(Box::new(WsSocket { socket }))
    }
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

/// A tunnel to `host:port` through the HTTP proxy `proxy` (CONNECT), its user and password sent when it has them.
async fn tunnel(
    proxy: &reqwest::Url,
    host: &str,
    port: u16,
) -> Result<tokio::net::TcpStream, ClientError> {
    use base64::Engine as _;
    let net = |e: std::io::Error| ClientError::Net(e.to_string());
    let p_host = proxy
        .host_str()
        .ok_or_else(|| ClientError::Net("the proxy has no host".into()))?;
    let p_port = proxy.port_or_known_default().unwrap_or(8080);
    let mut tcp = tokio::net::TcpStream::connect((p_host, p_port))
        .await
        .map_err(net)?;
    let mut head = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n");
    if !proxy.username().is_empty() {
        let user = format!(
            "{}:{}",
            proxy.username(),
            proxy.password().unwrap_or_default()
        );
        head.push_str(&format!(
            "Proxy-Authorization: Basic {}\r\n",
            base64::engine::general_purpose::STANDARD.encode(user)
        ));
    }
    head.push_str("\r\n");
    tcp.write_all(head.as_bytes()).await.map_err(net)?;
    // The answer's head, byte by byte: nothing of the tunnel's own bytes is read past it.
    let mut answer = Vec::new();
    while !answer.ends_with(b"\r\n\r\n") {
        if answer.len() > 16 << 10 {
            return Err(ClientError::Net("the proxy's answer is too long".into()));
        }
        let mut b = [0u8; 1];
        if tcp.read(&mut b).await.map_err(net)? == 0 {
            return Err(ClientError::Net("the proxy closed the tunnel".into()));
        }
        answer.push(b[0]);
    }
    let line = String::from_utf8_lossy(&answer);
    let status = line.split_whitespace().nth(1).unwrap_or("");
    if status != "200" {
        return Err(ClientError::Net(format!(
            "the proxy refused the tunnel to {host} ({status})"
        )));
    }
    Ok(tcp)
}

struct WsSocket {
    socket: tokio_tungstenite::WebSocketStream<Box<dyn Io>>,
}

#[async_trait]
impl LiveSocket for WsSocket {
    async fn next(&mut self) -> Option<Result<LiveMessage, ClientError>> {
        use tokio_tungstenite::tungstenite::Message;
        loop {
            let message = self.socket.next().await?;
            return Some(match message {
                Ok(Message::Binary(b)) => Ok(LiveMessage::Binary(b.to_vec())),
                Ok(Message::Close(frame)) => Ok(LiveMessage::Closed(frame.map(|f| f.code.into()))),
                Ok(Message::Ping(p)) => {
                    let _ = self.socket.send(Message::Pong(p)).await;
                    continue;
                }
                Ok(_) => continue,
                Err(e) => Err(ClientError::Net(e.to_string())),
            });
        }
    }
}

// === The client ===

struct Session {
    token: String,
    until: Instant,
}

/// What enrolment gives.
#[derive(Clone, Debug)]
pub struct Enrolled {
    pub team: [u8; 16],
    pub device: [u8; 16],
    pub role: Role,
}

/// The client of one device on one server.
pub struct ServerClient {
    transport: Arc<dyn Transport>,
    device: [u8; 16],
    key: SigningKey,
    session: Mutex<Option<Session>>,
}

fn id16(bytes: &[u8]) -> Result<[u8; 16], ClientError> {
    bytes
        .try_into()
        .map_err(|_| ClientError::Decode("an id is 16 bytes".into()))
}

/// The message the device signs for a challenge.
pub fn signed_message(device: &[u8; 16], challenge: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(AUTH_LABEL.len() + 48);
    out.extend_from_slice(AUTH_LABEL);
    out.extend_from_slice(device);
    out.extend_from_slice(challenge);
    out
}

/// An answer that is not a success, as an error.
fn refusal(reply: &Reply) -> ClientError {
    let problem: Option<Problem> = wire::decode(&reply.body).ok();
    match (reply.status, problem) {
        (426, _) => ClientError::ClientTooOld,
        (400, Some(p)) if p.error == "server_too_old" => ClientError::ServerTooOld,
        (409, Some(p)) if p.error == "epoch_taken" => {
            match p.detail.and_then(|d| d.deserialized::<EpochTaken>().ok()) {
                Some(detail) => ClientError::EpochTaken(detail),
                None => ClientError::Decode("an epoch_taken without its detail".into()),
            }
        }
        (409, Some(p))
            if matches!(
                p.error.as_str(),
                "taken" | "stale_version" | "claim_lost" | "stale_token"
            ) =>
        {
            match p.detail.and_then(|d| d.deserialized::<Claim>().ok()) {
                Some(claim) => ClientError::Claim {
                    code: p.error,
                    claim: Box::new(claim),
                },
                None => ClientError::Decode("a refused claim without its detail".into()),
            }
        }
        (status, Some(p)) => ClientError::Refused {
            status,
            error: p.error,
            message: p.message,
        },
        (status, None) => ClientError::Refused {
            status,
            error: String::new(),
            message: String::from_utf8_lossy(&reply.body).into_owned(),
        },
    }
}

fn ok_cbor<T: serde::de::DeserializeOwned>(reply: Reply) -> Result<T, ClientError> {
    if !(200..300).contains(&reply.status) {
        return Err(refusal(&reply));
    }
    wire::decode(&reply.body).map_err(ClientError::Decode)
}

/// A slot of the pairing relay, as it goes in a path: three characters of its alphabet.
fn slot_path(slot: &str) -> Result<&str, ClientError> {
    if slot.len() == 3 && slot.bytes().all(|b| b.is_ascii_alphanumeric()) {
        Ok(slot)
    } else {
        Err(ClientError::Decode(
            "a code's slot is three characters".into(),
        ))
    }
}

// === The pairing relay, the side of the new device (not enrolled: no session) ===

/// The first message of the slot.
pub async fn pair_message(transport: &dyn Transport, slot: &str) -> Result<Vec<u8>, ClientError> {
    let reply = transport
        .call(Call {
            method: Method::Get,
            path: format!("/v1/pair/{}", slot_path(slot)?),
            content_type: None,
            body: Vec::new(),
            token: None,
        })
        .await?;
    Ok(ok_cbor::<PairMessage>(reply)?.message)
}

/// The new device's one try.
pub async fn pair_try(
    transport: &dyn Transport,
    slot: &str,
    message: Vec<u8>,
    confirm: Vec<u8>,
) -> Result<(), ClientError> {
    let reply = transport
        .call(cbor_call(
            Method::Post,
            format!("/v1/pair/{}/try", slot_path(slot)?),
            &PairTry { message, confirm },
        ))
        .await?;
    match reply.status {
        204 => Ok(()),
        _ => Err(refusal(&reply)),
    }
}

/// The answer, once the device in the team left it (the slot goes with it); None while there is none.
pub async fn pair_take(
    transport: &dyn Transport,
    slot: &str,
) -> Result<Option<Vec<u8>>, ClientError> {
    let reply = transport
        .call(Call {
            method: Method::Get,
            path: format!("/v1/pair/{}/answer", slot_path(slot)?),
            content_type: None,
            body: Vec::new(),
            token: None,
        })
        .await?;
    if reply.status == 204 {
        return Ok(None);
    }
    Ok(Some(ok_cbor::<PairAnswer>(reply)?.sealed))
}

fn cbor_call(method: Method, path: String, body: &impl serde::Serialize) -> Call {
    Call {
        method,
        path,
        content_type: Some("application/cbor"),
        body: wire::encode(body),
        token: None,
    }
}

impl ServerClient {
    /// Enrols a device with a single-use token: the one of `init` creates the team, an administrator's adds a device
    /// to it. `key` is the device's server key.
    pub async fn enroll(
        transport: &dyn Transport,
        token: &str,
        key: &SigningKey,
    ) -> Result<Enrolled, ClientError> {
        let reply = transport
            .call(cbor_call(
                Method::Post,
                "/v1/enroll".into(),
                &EnrollRequest {
                    token: token.to_string(),
                    public_key: key.verifying_key().to_bytes().to_vec(),
                },
            ))
            .await?;
        let r: EnrollResponse = ok_cbor(reply)?;
        Ok(Enrolled {
            team: id16(&r.team)?,
            device: id16(&r.device)?,
            role: r.role,
        })
    }

    pub fn new(transport: Arc<dyn Transport>, device: [u8; 16], key: SigningKey) -> Self {
        Self {
            transport,
            device,
            key,
            session: Mutex::new(None),
        }
    }

    pub fn device(&self) -> [u8; 16] {
        self.device
    }

    /// Signs a new challenge and takes its session.
    pub async fn sign_in(&self) -> Result<(), ClientError> {
        let token = self.open_session().await?;
        *self.session.lock().await = Some(token);
        Ok(())
    }

    async fn open_session(&self) -> Result<Session, ClientError> {
        let reply = self
            .transport
            .call(cbor_call(
                Method::Post,
                "/v1/auth/challenge".into(),
                &ChallengeRequest {
                    device: self.device.to_vec(),
                },
            ))
            .await?;
        if reply.status == 401 {
            return Err(ClientError::SignInRefused);
        }
        let c: ChallengeResponse = ok_cbor(reply)?;
        let challenge: [u8; 32] = c
            .challenge
            .as_slice()
            .try_into()
            .map_err(|_| ClientError::Decode("a challenge is 32 bytes".into()))?;
        let signature = self
            .key
            .sign(&signed_message(&self.device, &challenge))
            .to_bytes()
            .to_vec();
        let reply = self
            .transport
            .call(cbor_call(
                Method::Post,
                "/v1/auth/session".into(),
                &SessionRequest {
                    device: self.device.to_vec(),
                    challenge: challenge.to_vec(),
                    signature,
                },
            ))
            .await?;
        if reply.status == 401 {
            return Err(ClientError::SignInRefused);
        }
        let t: TokenResponse = ok_cbor(reply)?;
        Ok(Session {
            token: t.token,
            until: Instant::now() + Duration::from_secs(t.expires_in),
        })
    }

    /// The session's token, renewed when it ends soon.
    pub async fn token(&self) -> Result<String, ClientError> {
        let mut session = self.session.lock().await;
        if let Some(s) = session.as_ref() {
            if s.until > Instant::now() + RENEW_BEFORE {
                return Ok(s.token.clone());
            }
        }
        let fresh = self.open_session().await?;
        let token = fresh.token.clone();
        *session = Some(fresh);
        Ok(token)
    }

    /// Forgets the session: the next call signs a new challenge.
    pub async fn forget_session(&self) {
        *self.session.lock().await = None;
    }

    /// A call with the session; refused as unauthorised (the server restarted, the session ended early), it signs in
    /// again once and sends it again.
    async fn authed(&self, call: Call) -> Result<Reply, ClientError> {
        for attempt in 0..2 {
            let mut c = call.clone();
            c.token = Some(self.token().await?);
            let reply = self.transport.call(c).await?;
            if reply.status == 401 && attempt == 0 {
                self.forget_session().await;
                continue;
            }
            return Ok(reply);
        }
        unreachable!("two attempts at most")
    }

    pub async fn devices(&self) -> Result<Vec<DeviceInfo>, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path: "/v1/devices".into(),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        Ok(ok_cbor::<Devices>(reply)?.devices)
    }

    /// An enrolment token for one more device (an administrator's).
    pub async fn enrollment_token(&self, expires_in: Option<u64>) -> Result<String, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                "/v1/devices/enrollments".into(),
                &EnrollmentRequest {
                    expires_in,
                    token_hash: None,
                },
            ))
            .await?;
        Ok(ok_cbor::<TokenResponse>(reply)?.token)
    }

    /// Registers the token whose SHA-256 is `hash` (an invitation's, made by this device) for `expires_in` seconds
    /// (the server keeps 7 days at most): an administrator's. Gives the time it was kept for.
    pub async fn register_token(
        &self,
        hash: [u8; 32],
        expires_in: u64,
    ) -> Result<u64, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                "/v1/devices/enrollments".into(),
                &EnrollmentRequest {
                    expires_in: Some(expires_in),
                    token_hash: Some(hash.to_vec()),
                },
            ))
            .await?;
        Ok(ok_cbor::<Registered>(reply)?.expires_in)
    }

    /// The team's tokens used a second time (an administrator's).
    pub async fn token_alerts(&self) -> Result<Vec<TokenAlertInfo>, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path: "/v1/devices/enrollments/alerts".into(),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        Ok(ok_cbor::<TokenAlerts>(reply)?.alerts)
    }

    /// Sets a device's role on the server (an administrator's): the server's roles kept in step with the roster.
    pub async fn set_role(&self, device: &[u8; 16], role: Role) -> Result<(), ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                format!("/v1/devices/{}/role", hex::encode(device)),
                &RoleRequest { role },
            ))
            .await?;
        match reply.status {
            204 => Ok(()),
            _ => Err(refusal(&reply)),
        }
    }

    // === The pairing relay (`pairing.rs`), the side of the device in the team ===

    pub async fn pair_open(&self, message: &[u8]) -> Result<PairOpened, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                "/v1/pair".into(),
                &PairOpen {
                    message: message.to_vec(),
                },
            ))
            .await?;
        ok_cbor(reply)
    }

    /// The new device's try, once it is there.
    pub async fn pair_tried(&self, slot: &str) -> Result<Option<PairTry>, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path: format!("/v1/pair/{}/try", slot_path(slot)?),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        if reply.status == 204 {
            return Ok(None);
        }
        ok_cbor(reply).map(Some)
    }

    pub async fn pair_answer(&self, slot: &str, sealed: Vec<u8>) -> Result<(), ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                format!("/v1/pair/{}/answer", slot_path(slot)?),
                &PairAnswer { sealed },
            ))
            .await?;
        match reply.status {
            204 => Ok(()),
            _ => Err(refusal(&reply)),
        }
    }

    pub async fn pair_close(&self, slot: &str) -> Result<(), ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Delete,
                path: format!("/v1/pair/{}", slot_path(slot)?),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        match reply.status {
            204 | 404 => Ok(()),
            _ => Err(refusal(&reply)),
        }
    }

    /// Revokes a device on the server: its sessions and live connections end. True when it was revoked now, false
    /// when the server no longer knows it as a device to revoke.
    pub async fn revoke(&self, device: &[u8; 16]) -> Result<bool, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Post,
                path: format!("/v1/devices/{}/revoke", hex::encode(device)),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        match reply.status {
            204 => Ok(true),
            404 => Ok(false),
            _ => Err(refusal(&reply)),
        }
    }

    async fn append(
        &self,
        path: String,
        kind: Kind,
        epoch: Option<u64>,
        payload: &[u8],
    ) -> Result<u64, ClientError> {
        self.append_with(path, kind, epoch, payload, None).await
    }

    async fn append_with(
        &self,
        path: String,
        kind: Kind,
        epoch: Option<u64>,
        payload: &[u8],
        fence: Option<FenceWire>,
    ) -> Result<u64, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                path,
                &AppendRequest {
                    kind,
                    epoch,
                    payload: payload.to_vec(),
                    fence,
                },
            ))
            .await?;
        Ok(ok_cbor::<Appended>(reply)?.seq)
    }

    /// A write of a ticket's holder in a project's stream (TS-6): refused with `ClientError::Claim` (`stale_token`)
    /// once another device took the ticket over.
    pub async fn append_project_fenced(
        &self,
        project: &[u8; 16],
        payload: &[u8],
        fence: Option<FenceWire>,
    ) -> Result<u64, ClientError> {
        self.append_with(
            format!("/v1/projects/{}/stream", hex::encode(project)),
            Kind::Message,
            None,
            payload,
            fence,
        )
        .await
    }

    /// Who has access to a project on the server (TS-6).
    pub async fn access(&self, project: &[u8; 16]) -> Result<AccessWire, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path: format!("/v1/projects/{}/access", hex::encode(project)),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        ok_cbor(reply)
    }

    /// Sets who has access to a project: any device for a project the server does not know yet, an administrator
    /// afterwards.
    pub async fn set_access(
        &self,
        project: &[u8; 16],
        access: &AccessWire,
    ) -> Result<(), ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Put,
                format!("/v1/projects/{}/access", hex::encode(project)),
                access,
            ))
            .await?;
        if (200..300).contains(&reply.status) {
            Ok(())
        } else {
            Err(refusal(&reply))
        }
    }

    /// Takes a ticket (§4.2), as it was at `expected` when given.
    pub async fn claim(
        &self,
        project: &[u8; 16],
        ticket: &[u8; 16],
        expected: Option<u64>,
    ) -> Result<ClaimAnswer, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                format!(
                    "/v1/projects/{}/claims/{}",
                    hex::encode(project),
                    hex::encode(ticket)
                ),
                &ClaimRequest { expected },
            ))
            .await?;
        ok_cbor(reply)
    }

    async fn claim_token(
        &self,
        project: &[u8; 16],
        ticket: &[u8; 16],
        what: &str,
        token: u64,
    ) -> Result<ClaimAnswer, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                format!(
                    "/v1/projects/{}/claims/{}/{what}",
                    hex::encode(project),
                    hex::encode(ticket)
                ),
                &ClaimToken { token },
            ))
            .await?;
        ok_cbor(reply)
    }

    /// Renews a lease, under its token.
    pub async fn renew_claim(
        &self,
        project: &[u8; 16],
        ticket: &[u8; 16],
        token: u64,
    ) -> Result<ClaimAnswer, ClientError> {
        self.claim_token(project, ticket, "renew", token).await
    }

    /// Gives a ticket back.
    pub async fn release_claim(
        &self,
        project: &[u8; 16],
        ticket: &[u8; 16],
        token: u64,
    ) -> Result<ClaimAnswer, ClientError> {
        self.claim_token(project, ticket, "release", token).await
    }

    /// Hands a ticket to another device of the project (TS-17): it holds it at once, under a new token, until it takes
    /// it or gives it back. `not_in_project` for a device revoked, without access, or this one.
    pub async fn hand_claim(
        &self,
        project: &[u8; 16],
        ticket: &[u8; 16],
        token: u64,
        to: &[u8; 16],
    ) -> Result<ClaimAnswer, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                format!(
                    "/v1/projects/{}/claims/{}/hand",
                    hex::encode(project),
                    hex::encode(ticket)
                ),
                &HandRequest {
                    token,
                    to: to.to_vec(),
                },
            ))
            .await?;
        ok_cbor(reply)
    }

    /// Takes back a ticket this device handed, under the token of the hand, while it was not taken.
    pub async fn withdraw_claim(
        &self,
        project: &[u8; 16],
        ticket: &[u8; 16],
        token: u64,
    ) -> Result<ClaimAnswer, ClientError> {
        self.claim_token(project, ticket, "withdraw", token).await
    }

    /// A project's claims, with the server's time.
    pub async fn claims(&self, project: &[u8; 16]) -> Result<Claims, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path: format!("/v1/projects/{}/claims", hex::encode(project)),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        ok_cbor(reply)
    }

    /// The next number of a project's tickets (§4.1).
    pub async fn next_key(&self, project: &[u8; 16]) -> Result<u64, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Post,
                format!("/v1/projects/{}/keys", hex::encode(project)),
                &(),
            ))
            .await?;
        Ok(ok_cbor::<KeyNumber>(reply)?.n)
    }

    async fn read(&self, path: String) -> Result<Entries, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path,
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        ok_cbor(reply)
    }

    /// A commit for `epoch`, in the team's stream: refused with [`ClientError::EpochTaken`] when another holds it.
    pub async fn append_commit(&self, epoch: u64, commit: &[u8]) -> Result<u64, ClientError> {
        self.append("/v1/team/stream".into(), Kind::Commit, Some(epoch), commit)
            .await
    }

    /// A message in the team's stream.
    pub async fn append_message(&self, payload: &[u8]) -> Result<u64, ClientError> {
        self.append("/v1/team/stream".into(), Kind::Message, None, payload)
            .await
    }

    /// The team's entries after `since`, a page at most (`more` says whether there are more).
    pub async fn read_team(&self, since: u64, limit: Option<u32>) -> Result<Entries, ClientError> {
        let mut path = format!("/v1/team/stream?since={since}");
        if let Some(l) = limit {
            path.push_str(&format!("&limit={l}"));
        }
        self.read(path).await
    }

    pub async fn append_project(
        &self,
        project: &[u8; 16],
        payload: &[u8],
    ) -> Result<u64, ClientError> {
        self.append(
            format!("/v1/projects/{}/stream", hex::encode(project)),
            Kind::Message,
            None,
            payload,
        )
        .await
    }

    pub async fn read_project(
        &self,
        project: &[u8; 16],
        since: u64,
        limit: Option<u32>,
    ) -> Result<Entries, ClientError> {
        let mut path = format!("/v1/projects/{}/stream?since={since}", hex::encode(project));
        if let Some(l) = limit {
            path.push_str(&format!("&limit={l}"));
        }
        self.read(path).await
    }

    /// A commit of a project's own group (TS-5) for `epoch`: refused with [`ClientError::EpochTaken`] when another
    /// holds it.
    pub async fn append_project_commit(
        &self,
        project: &[u8; 16],
        epoch: u64,
        commit: &[u8],
    ) -> Result<u64, ClientError> {
        self.append(
            format!("/v1/projects/{}/stream", hex::encode(project)),
            Kind::Commit,
            Some(epoch),
            commit,
        )
        .await
    }

    /// Sends a project's snapshot: the server forgets the messages up to `upto`. Gives how many it forgot.
    pub async fn put_snapshot(
        &self,
        project: &[u8; 16],
        upto: u64,
        payload: Vec<u8>,
    ) -> Result<u64, ClientError> {
        let reply = self
            .authed(cbor_call(
                Method::Put,
                format!("/v1/projects/{}/snapshot", hex::encode(project)),
                &SnapshotPut { upto, payload },
            ))
            .await?;
        Ok(ok_cbor::<SnapshotKept>(reply)?.pruned)
    }

    /// A project's last snapshot; None when it has none.
    pub async fn get_snapshot(
        &self,
        project: &[u8; 16],
    ) -> Result<Option<SnapshotInfo>, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path: format!("/v1/projects/{}/snapshot", hex::encode(project)),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        if reply.status == 404 {
            return Ok(None);
        }
        Ok(Some(ok_cbor(reply)?))
    }

    /// Writes an encrypted file, once (refused if it exists).
    pub async fn put_file(&self, id: &[u8; 16], bytes: Vec<u8>) -> Result<(), ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Put,
                path: format!("/v1/files/{}", hex::encode(id)),
                content_type: Some("application/octet-stream"),
                body: bytes,
                token: None,
            })
            .await?;
        if (200..300).contains(&reply.status) {
            Ok(())
        } else {
            Err(refusal(&reply))
        }
    }

    /// An encrypted file, None when the server has none.
    pub async fn get_file(&self, id: &[u8; 16]) -> Result<Option<Vec<u8>>, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path: format!("/v1/files/{}", hex::encode(id)),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        match reply.status {
            200 => Ok(Some(reply.body)),
            404 => Ok(None),
            _ => Err(refusal(&reply)),
        }
    }

    pub async fn delete_file(&self, id: &[u8; 16]) -> Result<(), ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Delete,
                path: format!("/v1/files/{}", hex::encode(id)),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        if (200..300).contains(&reply.status) || reply.status == 404 {
            Ok(())
        } else {
            Err(refusal(&reply))
        }
    }

    pub async fn usage(&self) -> Result<Usage, ClientError> {
        let reply = self
            .authed(Call {
                method: Method::Get,
                path: "/v1/files/usage".into(),
                content_type: None,
                body: Vec::new(),
                token: None,
            })
            .await?;
        ok_cbor(reply)
    }

    /// Subscribes to the live connection, which comes back by itself (`LiveFeed`).
    pub fn live(self: &Arc<Self>) -> LiveFeed {
        LiveFeed::start(self.clone(), LiveOptions::default())
    }

    pub fn live_with(self: &Arc<Self>, options: LiveOptions) -> LiveFeed {
        LiveFeed::start(self.clone(), options)
    }
}

// === The live connection ===

/// What the live feed says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveEvent {
    /// Connected, the first time or again: what came while it was away is read from the streams, from their last
    /// numbers (`Team::sync`).
    Connected,
    Team(Entry),
    Project([u8; 16], Entry),
    /// A ticket of a project taken, renewed or given back (TS-6).
    Claim([u8; 16], Claim),
    /// The server dropped frames for this device (it fell behind): read the streams again.
    Lagged,
    /// The device is revoked: the feed stops.
    Revoked,
}

#[derive(Clone, Copy, Debug)]
pub struct LiveOptions {
    /// The first wait before coming back, doubled at each failure up to `max_backoff`.
    pub backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for LiveOptions {
    fn default() -> Self {
        Self {
            backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
        }
    }
}

/// The live feed of a device: a task that holds the connection, opens it again after a cut (with a growing wait),
/// renews the session when the server ends it (4001), and stops when the device is revoked (4003). Dropped, it
/// stops.
pub struct LiveFeed {
    rx: mpsc::Receiver<LiveEvent>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for LiveFeed {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl LiveFeed {
    fn start(client: Arc<ServerClient>, options: LiveOptions) -> Self {
        let (tx, rx) = mpsc::channel(256);
        let task = tokio::spawn(async move {
            let mut wait = options.backoff;
            loop {
                match Self::session(&client, &tx).await {
                    Ok(Stop::Revoked) => {
                        let _ = tx.send(LiveEvent::Revoked).await;
                        return;
                    }
                    Ok(Stop::Gone) => return,
                    Ok(Stop::Connected) => wait = options.backoff,
                    Err(ClientError::SignInRefused) => {
                        let _ = tx.send(LiveEvent::Revoked).await;
                        return;
                    }
                    Err(e) => log::debug!("team: live connection: {e}"),
                }
                if tx.is_closed() {
                    return;
                }
                tokio::time::sleep(wait).await;
                wait = (wait * 2).min(options.max_backoff);
            }
        });
        Self { rx, task }
    }

    /// One connection, until it closes. `Connected` once it was up (the wait starts again from the first).
    async fn session(
        client: &Arc<ServerClient>,
        tx: &mpsc::Sender<LiveEvent>,
    ) -> Result<Stop, ClientError> {
        let token = client.token().await?;
        let mut socket = match client.transport.live("/v1/live", &token).await {
            Ok(s) => s,
            Err(ClientError::Refused { status: 401, .. }) => {
                client.forget_session().await;
                return Err(ClientError::Net("the session was refused".into()));
            }
            Err(e) => return Err(e),
        };
        if tx.send(LiveEvent::Connected).await.is_err() {
            return Ok(Stop::Gone);
        }
        while let Some(message) = socket.next().await {
            let event = match message {
                Ok(LiveMessage::Binary(bytes)) => match wire::decode::<Frame>(&bytes) {
                    Ok(f) => match (f.stream.as_str(), f.project, f.entry) {
                        ("claim", Some(p), _) => match (p.as_slice().try_into(), f.claim) {
                            (Ok(p), Some(c)) => LiveEvent::Claim(p, c),
                            _ => continue,
                        },
                        ("team", _, Some(entry)) => LiveEvent::Team(entry),
                        ("project", Some(p), Some(entry)) => match p.as_slice().try_into() {
                            Ok(p) => LiveEvent::Project(p, entry),
                            Err(_) => continue,
                        },
                        ("lagged", _, _) => LiveEvent::Lagged,
                        _ => continue,
                    },
                    Err(_) => continue,
                },
                Ok(LiveMessage::Closed(Some(4003))) => return Ok(Stop::Revoked),
                Ok(LiveMessage::Closed(Some(4001))) => {
                    client.forget_session().await;
                    return Ok(Stop::Connected);
                }
                Ok(LiveMessage::Closed(_)) => return Ok(Stop::Connected),
                Err(e) => {
                    log::debug!("team: live connection cut: {e}");
                    return Ok(Stop::Connected);
                }
            };
            if tx.send(event).await.is_err() {
                return Ok(Stop::Gone);
            }
        }
        Ok(Stop::Connected)
    }

    /// The next event; None once the feed stopped.
    pub async fn next(&mut self) -> Option<LiveEvent> {
        self.rx.recv().await
    }
}

enum Stop {
    Connected,
    Revoked,
    Gone,
}
