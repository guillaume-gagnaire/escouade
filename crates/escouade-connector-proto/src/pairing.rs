//! The pairing (§ 4.5): how a connector and a server, that know nothing of each other but a code of 8 letters given by
//! the server's space, learn each other's static key.
//!
//! - **The way**: the same entry as the channels, at its path followed by [`SUFFIX`] (`wss://…:8443/connector/pair`,
//!   [`pair_path`], [`pairing_url`]), through the company's proxy as the channels.
//! - **The handshake**: `Noise_XXpsk0_25519_ChaChaPoly_BLAKE2s` ([`PATTERN`]), with [`PROLOGUE`], the pre-shared key
//!   being BLAKE2s-256 of [`PROLOGUE`] followed by the code put in shape ([`normalize_code`], [`CodeKey`]). The code
//!   never travels, not even under an inspection of TLS; a wrong code does not decrypt the first message, and the
//!   server closes without a word (an attempt that failed, for the bounds of the entry). Each side proves it holds the
//!   private half of the static key it gives (XX).
//! - **The messages**: the three of XX in the WebSocket's first three binary messages (the first carries the
//!   connector's [`Hello`], the second the server's [`Welcome`], the third nothing), then a fourth from the server,
//!   sealed: [`PAIRED`] alone when the server pinned the connector's key, [`REFUSED`] followed by a reason in UTF-8
//!   otherwise (a version refused, with [`Welcome::advice`]; a refusal of the server's caller). The connector keeps the
//!   server's key only after [`PAIRED`].
//!
//! Neither the code nor a private key is in an error, a `Debug` or a journal: [`CodeKey`] shows nothing of itself.

use std::fmt;
use std::io;
use std::time::Duration;

use blake2::{Blake2s256, Digest};
use futures_util::{SinkExt, StreamExt};
use snow::params::NoiseParams;
use snow::HandshakeState;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use zeroize::{Zeroize, Zeroizing};

use crate::noise::{Keypair, PublicKey, MAX_MESSAGE};
use crate::version::{Hello, Verdict, Version, Welcome};

/// The pattern of the pairing, its curve, its cipher and its hash.
pub const PATTERN: &str = "Noise_XXpsk0_25519_ChaChaPoly_BLAKE2s";
/// Bound into both sides' handshake, and into the pre-shared key.
pub const PROLOGUE: &[u8] = b"escouade-connector-pair/1";
/// What follows the entry's path for the pairing: `/connector` → `/connector/pair`.
pub const SUFFIX: &str = "/pair";
/// The fourth message: the connector's key is pinned.
pub const PAIRED: u8 = 0x01;
/// The fourth message: refused, a reason in UTF-8 follows.
pub const REFUSED: u8 = 0x00;
/// The most bytes of a reason in the fourth message.
pub const MAX_REASON: usize = 1024;

fn params() -> NoiseParams {
    PATTERN.parse().expect("the pattern is valid")
}

/// The path of the pairing for an entry served at `path` (`/connector` → `/connector/pair`; `/` or empty →
/// `/connector/pair`).
pub fn pair_path(path: &str) -> String {
    let base = path.trim_end_matches('/');
    let base = if base.is_empty() { crate::PATH } else { base };
    format!("{base}{SUFFIX}")
}

/// The pairing's address for the entry's address (`wss://host:8443/connector` → `wss://host:8443/connector/pair`).
pub fn pairing_url(url: &str) -> Option<String> {
    let mut url = url::Url::parse(url.trim()).ok()?;
    if url.scheme() != "wss" && url.scheme() != "ws" {
        return None;
    }
    let path = pair_path(url.path());
    url.set_path(&path);
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

/// A code put in shape: upper case, without dashes nor blanks (« 8nuc-w6hs » is « 8NUC-W6HS »).
pub fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

/// The pre-shared key of a code: what the server may keep of a pending code instead of the code itself. Its `Debug`
/// shows nothing; it is wiped when dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct CodeKey([u8; 32]);

impl CodeKey {
    /// BLAKE2s-256 of [`PROLOGUE`] followed by the code put in shape.
    pub fn from_code(code: &str) -> CodeKey {
        let code = Zeroizing::new(normalize_code(code));
        let mut hash = Blake2s256::new();
        hash.update(PROLOGUE);
        hash.update(code.as_bytes());
        let mut out = [0u8; 32];
        out.copy_from_slice(&hash.finalize());
        CodeKey(out)
    }

    /// A key kept by the caller (the bytes [`CodeKey::as_bytes`] gave).
    pub fn from_bytes(bytes: [u8; 32]) -> CodeKey {
        CodeKey(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Drop for CodeKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for CodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CodeKey(…)")
    }
}

/// What the connector learns from a pairing.
#[derive(Clone, Debug)]
pub struct Paired {
    /// The server's static key, to pin.
    pub server: PublicKey,
    pub welcome: Welcome,
}

/// Why the connector's pairing fails.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum PairError {
    #[error("the server cannot be reached: {0}")]
    Net(String),
    #[error("the server did not answer the pairing in time")]
    Timeout,
    /// What a server does with a code it does not know: closed without a word.
    #[error(
        "the server closed the pairing: the code is wrong, expired or already used (ask a new one)"
    )]
    WrongCode,
    #[error("the server's answer is not understood")]
    BadAnswer,
    /// The server refused this connector's version; [`Welcome::advice`] says what to do.
    #[error("{}", .welcome.advice(*.ours).unwrap_or_default())]
    Version { welcome: Welcome, ours: Version },
    /// The server refused, with its reason.
    #[error("the server refused the pairing: {0}")]
    Refused(String),
}

/// What the server learns from a pairing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Accepted {
    /// The index, in the pending codes, of the one used.
    pub index: usize,
    /// The connector's static key, pinned by `decide`.
    pub connector: PublicKey,
    pub version: Version,
}

/// Why the server's side of a pairing fails.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum PairAcceptError {
    #[error("no pairing in time")]
    Timeout,
    #[error("the network: {0}")]
    Network(String),
    /// Not a handshake of the pairing, or one whose later messages do not decrypt.
    #[error("not a pairing of the connectors' protocol")]
    NotNoise,
    /// No pending code decrypts the first message (none pending counts as such): closed without a word.
    #[error("a code no pairing waits for")]
    WrongCode,
    /// A version refused: told to the connector with the advice; `decide` not called, the code not used.
    #[error("escouade-connector {connector} refused ({verdict:?})")]
    Version {
        connector: Version,
        verdict: Verdict,
    },
    /// `decide` refused, with its reason (told to the connector).
    #[error("the pairing refused: {0}")]
    Refused(String),
}

impl PairAcceptError {
    /// Whether it counts as a failed try for the bounds of the entry (§ 4.5): a wrong code, no handshake in time, not
    /// this protocol. A version refused or a refusal of `decide` comes from someone who knew the code; a network error
    /// from no one in particular.
    pub fn is_failed_attempt(&self) -> bool {
        matches!(
            self,
            PairAcceptError::Timeout | PairAcceptError::NotNoise | PairAcceptError::WrongCode
        )
    }
}

type Ws<S> = WebSocketStream<S>;

/// The next binary message: `None` when the peer closed, with a close or without.
async fn next_binary<S>(ws: &mut Ws<S>) -> Result<Option<Vec<u8>>, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    use tokio_tungstenite::tungstenite::error::{Error, ProtocolError};
    loop {
        match ws.next().await {
            None | Some(Ok(Message::Close(_))) => return Ok(None),
            Some(Ok(Message::Binary(b))) => {
                if b.len() > MAX_MESSAGE {
                    return Err("a message too large".into());
                }
                return Ok(Some(b.to_vec()));
            }
            Some(Ok(Message::Text(_))) => return Err("a text message".into()),
            Some(Ok(_)) => continue,
            Some(Err(Error::ConnectionClosed | Error::AlreadyClosed))
            | Some(Err(Error::Protocol(ProtocolError::ResetWithoutClosingHandshake))) => {
                return Ok(None)
            }
            Some(Err(Error::Io(e)))
                if matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionReset
                        | io::ErrorKind::UnexpectedEof
                        | io::ErrorKind::BrokenPipe
                ) =>
            {
                return Ok(None)
            }
            Some(Err(e)) => return Err(e.to_string()),
        }
    }
}

fn builder<'a>(keys: &'a Keypair, psk: &'a CodeKey) -> Result<snow::Builder<'a>, snow::Error> {
    snow::Builder::new(params())
        .local_private_key(&keys.private_bytes()[..])?
        .psk(0, psk.as_bytes())?
        .prologue(PROLOGUE)
}

fn write(state: &mut HandshakeState, payload: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![0u8; MAX_MESSAGE];
    let n = state.write_message(payload, &mut out).ok()?;
    out.truncate(n);
    Some(out)
}

fn read(state: &mut HandshakeState, message: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![0u8; MAX_MESSAGE];
    let n = state.read_message(message, &mut out).ok()?;
    out.truncate(n);
    Some(out)
}

fn refusal(reason: &str) -> Vec<u8> {
    let mut end = reason.len().min(MAX_REASON);
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = vec![REFUSED];
    out.extend_from_slice(&reason.as_bytes()[..end]);
    out
}

/// The connector's side: pairs with the server at the other end of `ws` (opened on [`pairing_url`]) by `code`, with
/// the connector's `keys` (made for it, kept only after this succeeds). The whole exchange within `timeout`.
pub async fn pair_server<S>(
    ws: Ws<S>,
    keys: &Keypair,
    code: &str,
    version: Version,
    timeout: Duration,
) -> Result<Paired, PairError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    tokio::time::timeout(timeout, pair_now(ws, keys, code, version))
        .await
        .map_err(|_| PairError::Timeout)?
}

async fn pair_now<S>(
    mut ws: Ws<S>,
    keys: &Keypair,
    code: &str,
    version: Version,
) -> Result<Paired, PairError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let psk = CodeKey::from_code(code);
    let mut state = builder(keys, &psk)
        .and_then(|b| b.build_initiator())
        .map_err(|_| PairError::BadAnswer)?;
    let net = |e: tokio_tungstenite::tungstenite::Error| PairError::Net(e.to_string());
    let first = write(&mut state, &Hello { version }.encode()).ok_or(PairError::BadAnswer)?;
    ws.send(Message::Binary(first.into())).await.map_err(net)?;
    let second = next_binary(&mut ws)
        .await
        .map_err(PairError::Net)?
        .ok_or(PairError::WrongCode)?;
    let payload = read(&mut state, &second).ok_or(PairError::BadAnswer)?;
    let welcome = Welcome::decode(&payload).map_err(|_| PairError::BadAnswer)?;
    let third = write(&mut state, &[]).ok_or(PairError::BadAnswer)?;
    ws.send(Message::Binary(third.into())).await.map_err(net)?;
    let server = state
        .get_remote_static()
        .and_then(PublicKey::from_slice)
        .ok_or(PairError::BadAnswer)?;
    let mut transport = state
        .into_transport_mode()
        .map_err(|_| PairError::BadAnswer)?;
    let fourth = next_binary(&mut ws)
        .await
        .map_err(PairError::Net)?
        .ok_or_else(|| PairError::Net("the server closed before it confirmed".into()))?;
    let mut out = vec![0u8; MAX_MESSAGE];
    let n = transport
        .read_message(&fourth, &mut out)
        .map_err(|_| PairError::BadAnswer)?;
    let _ = ws.close(None).await;
    match out[..n].split_first() {
        Some((&PAIRED, [])) if welcome.verdict.accepted() => Ok(Paired { server, welcome }),
        Some((&REFUSED, _)) if !welcome.verdict.accepted() => Err(PairError::Version {
            welcome,
            ours: version,
        }),
        Some((&REFUSED, reason)) => Err(PairError::Refused(
            String::from_utf8_lossy(reason).into_owned(),
        )),
        _ => Err(PairError::BadAnswer),
    }
}

/// The server's side: reads the first message with each of the `pending` codes' keys (none decrypts: the socket is
/// dropped there, nothing said); answers its version's verdict; once the connector proved its key, asks `decide`
/// (the index of the code, the connector's key and version) to pin it, and says the outcome in the fourth message.
/// A version refused is told without calling `decide`: the code stays unused. The whole exchange within `timeout`.
pub async fn accept_pairing<S, F>(
    ws: Ws<S>,
    keys: &Keypair,
    version: Version,
    pending: &[CodeKey],
    decide: F,
    timeout: Duration,
) -> Result<Accepted, PairAcceptError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnOnce(usize, &PublicKey, Version) -> Result<(), String>,
{
    tokio::time::timeout(timeout, accept_now(ws, keys, version, pending, decide))
        .await
        .map_err(|_| PairAcceptError::Timeout)?
}

async fn accept_now<S, F>(
    mut ws: Ws<S>,
    keys: &Keypair,
    version: Version,
    pending: &[CodeKey],
    decide: F,
) -> Result<Accepted, PairAcceptError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnOnce(usize, &PublicKey, Version) -> Result<(), String>,
{
    let net = |e: tokio_tungstenite::tungstenite::Error| PairAcceptError::Network(e.to_string());
    let first = next_binary(&mut ws)
        .await
        .map_err(PairAcceptError::Network)?
        .ok_or(PairAcceptError::NotNoise)?;
    let mut found = None;
    for (index, psk) in pending.iter().enumerate() {
        let Ok(mut state) = builder(keys, psk).and_then(|b| b.build_responder()) else {
            continue;
        };
        if let Some(payload) = read(&mut state, &first) {
            found = Some((index, state, payload));
            break;
        }
    }
    let Some((index, mut state, payload)) = found else {
        // Nothing said: the socket dropped, not even a close.
        drop(ws);
        return Err(PairAcceptError::WrongCode);
    };
    let hello = Hello::decode(&payload).map_err(|_| PairAcceptError::NotNoise)?;
    let welcome = Welcome::for_hello(version, &hello);
    let second = write(&mut state, &welcome.encode()).ok_or(PairAcceptError::NotNoise)?;
    ws.send(Message::Binary(second.into())).await.map_err(net)?;
    let third = next_binary(&mut ws)
        .await
        .map_err(PairAcceptError::Network)?
        .ok_or(PairAcceptError::NotNoise)?;
    read(&mut state, &third).ok_or(PairAcceptError::NotNoise)?;
    let connector = state
        .get_remote_static()
        .and_then(PublicKey::from_slice)
        .ok_or(PairAcceptError::NotNoise)?;
    let mut transport = state
        .into_transport_mode()
        .map_err(|_| PairAcceptError::NotNoise)?;
    let (answer, outcome) = if !welcome.verdict.accepted() {
        let advice = welcome.advice(hello.version).unwrap_or_default();
        (
            refusal(&advice),
            Err(PairAcceptError::Version {
                connector: hello.version,
                verdict: welcome.verdict,
            }),
        )
    } else {
        match decide(index, &connector, hello.version) {
            Ok(()) => (
                vec![PAIRED],
                Ok(Accepted {
                    index,
                    connector,
                    version: hello.version,
                }),
            ),
            Err(reason) => (refusal(&reason), Err(PairAcceptError::Refused(reason))),
        }
    };
    let mut out = vec![0u8; answer.len() + crate::noise::TAG];
    let n = transport
        .write_message(&answer, &mut out)
        .map_err(|_| PairAcceptError::NotNoise)?;
    out.truncate(n);
    ws.send(Message::Binary(out.into())).await.map_err(net)?;
    let _ = ws.close(None).await;
    // The connector's close, so that it reads the fourth message before the socket goes.
    let _ = tokio::time::timeout(Duration::from_secs(2), next_binary(&mut ws)).await;
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::ws_config;
    use std::sync::{Arc, Mutex};
    use tokio::io::DuplexStream;
    use tokio_tungstenite::tungstenite::protocol::Role;

    const V: Version = Version::new(2, 3, 0);
    const T: Duration = Duration::from_secs(10);

    async fn sockets() -> (Ws<DuplexStream>, Ws<DuplexStream>) {
        let (a, b) = tokio::io::duplex(64 << 10);
        let server = WebSocketStream::from_raw_socket(a, Role::Server, Some(ws_config())).await;
        let client = WebSocketStream::from_raw_socket(b, Role::Client, Some(ws_config())).await;
        (server, client)
    }

    #[test]
    fn the_code_and_the_path_are_put_in_shape() {
        assert_eq!(normalize_code(" 8nuc-w6hs "), "8NUC-W6HS".replace('-', ""));
        assert_eq!(
            CodeKey::from_code("8nuc w6hs"),
            CodeKey::from_code("8NUC-W6HS")
        );
        assert_ne!(
            CodeKey::from_code("8NUC-W6HS"),
            CodeKey::from_code("8NUC-W6HT")
        );
        assert_eq!(
            format!("{:?}", CodeKey::from_code("8NUC-W6HS")),
            "CodeKey(…)"
        );
        assert_eq!(pair_path("/connector"), "/connector/pair");
        assert_eq!(pair_path("/connector/"), "/connector/pair");
        assert_eq!(pair_path("/"), "/connector/pair");
        assert_eq!(
            pairing_url("wss://escouade.exemple.fr:8443/connector").as_deref(),
            Some("wss://escouade.exemple.fr:8443/connector/pair")
        );
        assert_eq!(pairing_url("https://x/connector"), None);
    }

    #[tokio::test]
    async fn a_good_code_pins_both_keys() {
        let (server_ws, client_ws) = sockets().await;
        let (server_keys, connector_keys) = (Keypair::generate(), Keypair::generate());
        let server_public = server_keys.public();
        let connector_public = connector_keys.public();
        let pending = vec![
            CodeKey::from_code("AAAA-BBBB"),
            CodeKey::from_code("8NUC-W6HS"),
        ];
        let seen = Arc::new(Mutex::new(None));
        let seen2 = seen.clone();
        let server = tokio::spawn(async move {
            accept_pairing(
                server_ws,
                &server_keys,
                V,
                &pending,
                move |i, k, v| {
                    *seen2.lock().unwrap() = Some((i, *k, v));
                    Ok(())
                },
                T,
            )
            .await
        });
        let paired = pair_server(client_ws, &connector_keys, "8nuc-w6hs", V, T)
            .await
            .unwrap();
        assert_eq!(paired.server, server_public);
        assert_eq!(paired.welcome.verdict, Verdict::Current);
        let accepted = server.await.unwrap().unwrap();
        assert_eq!(
            accepted,
            Accepted {
                index: 1,
                connector: connector_public,
                version: V
            }
        );
        assert_eq!(*seen.lock().unwrap(), Some((1, connector_public, V)));
    }

    #[tokio::test]
    async fn a_wrong_code_or_none_pending_closes_without_a_word() {
        for pending in [vec![CodeKey::from_code("AAAA-BBBB")], vec![]] {
            let (server_ws, client_ws) = sockets().await;
            let server_keys = Keypair::generate();
            let server = tokio::spawn(async move {
                accept_pairing(server_ws, &server_keys, V, &pending, |_, _, _| Ok(()), T).await
            });
            let err = pair_server(client_ws, &Keypair::generate(), "8NUC-W6HS", V, T)
                .await
                .unwrap_err();
            assert_eq!(err, PairError::WrongCode);
            let err = server.await.unwrap().unwrap_err();
            assert_eq!(err, PairAcceptError::WrongCode);
            assert!(err.is_failed_attempt());
        }
    }

    #[tokio::test]
    async fn a_version_refused_is_told_and_decide_is_not_called() {
        let (server_ws, client_ws) = sockets().await;
        let server_keys = Keypair::generate();
        let pending = vec![CodeKey::from_code("8NUC-W6HS")];
        let server = tokio::spawn(async move {
            accept_pairing(
                server_ws,
                &server_keys,
                V,
                &pending,
                |_, _, _| panic!("decide is not called for a version refused"),
                T,
            )
            .await
        });
        let old = Version::new(2, 0, 0);
        let err = pair_server(client_ws, &Keypair::generate(), "8NUC-W6HS", old, T)
            .await
            .unwrap_err();
        match &err {
            PairError::Version { welcome, ours } => {
                assert_eq!(welcome.verdict, Verdict::TooOld);
                assert_eq!(*ours, old);
            }
            other => panic!("{other:?}"),
        }
        assert!(err.to_string().contains("update --to 2.3.0"), "{err}");
        let err = server.await.unwrap().unwrap_err();
        assert_eq!(
            err,
            PairAcceptError::Version {
                connector: old,
                verdict: Verdict::TooOld
            }
        );
        assert!(!err.is_failed_attempt());
    }

    #[tokio::test]
    async fn a_refusal_of_decide_reaches_the_connector() {
        let (server_ws, client_ws) = sockets().await;
        let server_keys = Keypair::generate();
        let pending = vec![CodeKey::from_code("8NUC-W6HS")];
        let server = tokio::spawn(async move {
            accept_pairing(
                server_ws,
                &server_keys,
                V,
                &pending,
                |_, _, _| Err("the code expired".into()),
                T,
            )
            .await
        });
        let err = pair_server(client_ws, &Keypair::generate(), "8NUC-W6HS", V, T)
            .await
            .unwrap_err();
        assert_eq!(err, PairError::Refused("the code expired".into()));
        let err = server.await.unwrap().unwrap_err();
        assert_eq!(err, PairAcceptError::Refused("the code expired".into()));
        assert!(!err.is_failed_attempt());
    }

    #[tokio::test(start_paused = true)]
    async fn silence_is_a_timeout_and_a_failed_attempt() {
        let (server_ws, _client_ws) = sockets().await;
        let err = accept_pairing(
            server_ws,
            &Keypair::generate(),
            V,
            &[CodeKey::from_code("8NUC-W6HS")],
            |_, _, _| Ok(()),
            T,
        )
        .await
        .unwrap_err();
        assert_eq!(err, PairAcceptError::Timeout);
        assert!(err.is_failed_attempt());
        let (_server_ws, client_ws) = sockets().await;
        // A server that reads and never answers.
        let err = pair_server(client_ws, &Keypair::generate(), "8NUC-W6HS", V, T)
            .await
            .unwrap_err();
        assert_eq!(err, PairError::Timeout);
    }

    #[tokio::test]
    async fn garbage_is_not_a_pairing() {
        let (server_ws, mut client_ws) = sockets().await;
        let server = tokio::spawn(async move {
            accept_pairing(
                server_ws,
                &Keypair::generate(),
                V,
                &[CodeKey::from_code("8NUC-W6HS")],
                |_, _, _| Ok(()),
                T,
            )
            .await
        });
        client_ws
            .send(Message::Binary(vec![7u8; 80].into()))
            .await
            .unwrap();
        let err = server.await.unwrap().unwrap_err();
        assert_eq!(err, PairAcceptError::WrongCode);
        assert!(err.is_failed_attempt());
    }
}
