//! A channel: the handshake on a WebSocket, then the streams.
//!
//! - [`connect_server`] (the connector) and [`accept_connector`] (the server) run the Noise handshake over the
//!   WebSocket's first two messages; the server checks the connector's key before it answers anything, and closes
//!   without a word when it does not know it ([`AcceptError::is_failed_attempt`]).
//! - Then one task per channel owns the WebSocket and the ciphers: the server [`ServerChannel::open`]s streams, the
//!   connector takes them by [`ConnectorChannel::next_open`] and [`Incoming::accept`]s or [`Incoming::refuse`]s each.
//!   A [`Stream`] is an `AsyncRead + AsyncWrite` (for `tokio::io::copy_bidirectional`).
//! - **The window**: each side grants [`INITIAL_WINDOW`] bytes per stream and direction, and grants again what its
//!   reader has read; a writer waits when its window is spent, a reader that does not read holds its own stream alone.
//!   A peer that sends beyond its window is closed (a protocol error), as one that sends a frame beyond the bound.
//! - **The life**: a `ping` every [`PING_EVERY`]; nothing heard for [`DEAD_AFTER`], the channel is cut
//!   ([`Closed::Silent`]); every stream is then reset, none is taken up again.
//! - **The bounds** ([`Limits`]): the streams open at once and the opens per second (one more is refused, `limits`),
//!   a stream silent for `stream_idle` closed, the size of a frame.
//!
//! The task writes through a task of its own, so that it never waits on the network: both peers can send at once
//! without either stopping to read.

use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::{Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::frame::{Ceiling, DecodeError, Frame, Open, Refusal, RefusalCode, StreamId};
use crate::limits::{
    Limits, Rate, DATA_CHUNK, DEAD_AFTER, INITIAL_WINDOW, MAX_CEILING_FRAME, MAX_WINDOW, PING_EVERY,
};
use crate::noise::{HandshakeError, Initiator, Keypair, PublicKey, Responder, Transport, TAG};
use crate::version::{Hello, Verdict, Version, Welcome};

/// Why a channel ended.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum Closed {
    #[error("closed on this side")]
    Local,
    #[error("closed by the peer")]
    ByPeer,
    /// Nothing heard for 45 s.
    #[error("nothing heard from the peer for {} s", DEAD_AFTER.as_secs())]
    Silent,
    #[error("the peer broke the protocol: {0}")]
    Protocol(String),
    #[error("the network: {0}")]
    Network(String),
}

/// Why the connector's connection fails.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ConnectError {
    /// The proxy, TLS, the WebSocket's upgrade: the way to the server.
    #[error("the server cannot be reached: {0}")]
    Net(String),
    #[error("the server did not answer the handshake in time")]
    Timeout,
    /// What a server does to a key it does not know: this connector is not (or no more) paired with it.
    #[error("the server closed the channel at the handshake: this connector is not paired with it, or no more")]
    NotPaired,
    /// The answer is not from the key pinned at the pairing: the server's keys changed, or something plays it.
    #[error(
        "the answer is not from the server paired (its key changed, or something plays the server)"
    )]
    NotTheServer,
    #[error("the server's answer is not understood")]
    BadAnswer,
    /// The server refused this connector's version; [`Welcome::advice`] says what to do.
    #[error("{}", .welcome.advice(*.ours).unwrap_or_default())]
    Version { welcome: Welcome, ours: Version },
}

/// Why the server refuses a connection, before any channel.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum AcceptError {
    #[error("no handshake in time")]
    Timeout,
    #[error("the network: {0}")]
    Network(String),
    /// Not a first message of this protocol, or one made for another server's key.
    #[error("not a handshake of the connectors' protocol")]
    NotNoise,
    #[error("a handshake whose payload is not understood")]
    BadPayload,
    /// A first message from a key no connector was paired with: closed without a word.
    #[error("a key no connector is paired with: {0}")]
    UnknownKey(PublicKey),
    /// A paired connector of a version refused: it was told the verdict (it is not an attempt that failed).
    #[error("escouade-connector {connector} refused ({verdict:?})")]
    Version {
        connector: Version,
        verdict: Verdict,
    },
}

impl AcceptError {
    /// Whether it counts as a failed try for the bounds of the entry (RP-4: 20 a minute, then all closed to the end of
    /// the minute, § 4.5): no handshake, not this protocol, an unknown key. A version refused comes from a key that is
    /// paired, and a network error from no one in particular.
    pub fn is_failed_attempt(&self) -> bool {
        matches!(
            self,
            AcceptError::Timeout
                | AcceptError::NotNoise
                | AcceptError::BadPayload
                | AcceptError::UnknownKey(_)
        )
    }
}

/// Why a stream is not opened.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum OpenError {
    /// The connector refused it, with its reason.
    #[error("{0}")]
    Refused(Refusal),
    /// This side's bounds: too many streams open, or too many opens in this second.
    #[error("too many streams on the channel")]
    Limits,
    /// A host, an agent or a project too long for the frame.
    #[error("a host, an agent or a project too long")]
    Invalid,
    #[error("the channel is closed")]
    Closed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum CeilingError {
    #[error("the ceiling is too large for a frame ({MAX_CEILING_FRAME} bytes at most)")]
    TooLarge,
    #[error("the channel is closed")]
    Closed,
}

type Ws<S> = WebSocketStream<S>;

/// The next binary message of the handshake: `None` when the peer closed, with a close or without (what a server
/// does to a key it does not know).
async fn next_binary<S>(ws: &mut Ws<S>) -> Result<Option<Vec<u8>>, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    use tokio_tungstenite::tungstenite::error::{Error, ProtocolError};
    loop {
        match ws.next().await {
            None | Some(Ok(Message::Close(_))) => return Ok(None),
            Some(Ok(Message::Binary(b))) => return Ok(Some(b.to_vec())),
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

/// The connector's side: the handshake with the server whose key is `server` (pinned at the pairing), then its
/// ceiling, sent at each connection. A version the server refuses is [`ConnectError::Version`]; one it accepts but
/// calls outdated is in [`ConnectorChannel::welcome`].
pub async fn connect_server<S>(
    mut ws: Ws<S>,
    keys: &Keypair,
    server: &PublicKey,
    version: Version,
    ceiling: &Ceiling,
    limits: Limits,
) -> Result<ConnectorChannel, ConnectError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let ceiling_frame =
        ceiling_frame(ceiling).map_err(|_| ConnectError::Net("the ceiling is too large".into()))?;
    let (initiator, first) =
        Initiator::start(keys, server, &Hello { version }).map_err(|_| ConnectError::BadAnswer)?;
    ws.send(Message::Binary(first.into()))
        .await
        .map_err(|e| ConnectError::Net(e.to_string()))?;
    let answer = tokio::time::timeout(limits.handshake_timeout, next_binary(&mut ws))
        .await
        .map_err(|_| ConnectError::Timeout)?
        .map_err(ConnectError::Net)?
        .ok_or(ConnectError::NotPaired)?;
    let (welcome, transport) = initiator.finish(&answer).map_err(|e| match e {
        HandshakeError::BadPayload => ConnectError::BadAnswer,
        _ => ConnectError::NotTheServer,
    })?;
    if !welcome.verdict.accepted() {
        let _ = ws.close(None).await;
        return Err(ConnectError::Version {
            welcome,
            ours: version,
        });
    }
    let (incoming_tx, incoming) = mpsc::unbounded_channel();
    let core = spawn(ws, transport, Role::Connector, limits, Some(incoming_tx));
    let _ = core.cmd.send(Cmd::Ceiling(ceiling_frame));
    Ok(ConnectorChannel {
        core,
        incoming,
        welcome,
    })
}

/// The server's side: reads the first message, asks `known` whether the connector's key is pinned (none known: the
/// connection is dropped there, nothing said), answers its version's verdict, and starts the channel.
pub async fn accept_connector<S, F>(
    mut ws: Ws<S>,
    keys: &Keypair,
    known: F,
    version: Version,
    limits: Limits,
) -> Result<ServerChannel, AcceptError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    F: FnOnce(&PublicKey) -> bool,
{
    let first = tokio::time::timeout(limits.handshake_timeout, next_binary(&mut ws))
        .await
        .map_err(|_| AcceptError::Timeout)?
        .map_err(AcceptError::Network)?
        .ok_or(AcceptError::NotNoise)?;
    let responder = Responder::read(keys, &first).map_err(|e| match e {
        HandshakeError::BadPayload => AcceptError::BadPayload,
        _ => AcceptError::NotNoise,
    })?;
    let peer = responder.remote();
    if !known(&peer) {
        // Nothing said: the socket dropped, not even a close.
        drop(ws);
        return Err(AcceptError::UnknownKey(peer));
    }
    let hello = responder.hello();
    let welcome = Welcome::for_hello(version, &hello);
    let (second, transport) = responder
        .respond(&welcome)
        .map_err(|_| AcceptError::NotNoise)?;
    ws.send(Message::Binary(second.into()))
        .await
        .map_err(|e| AcceptError::Network(e.to_string()))?;
    if !welcome.verdict.accepted() {
        let _ = ws.close(None).await;
        return Err(AcceptError::Version {
            connector: hello.version,
            verdict: welcome.verdict,
        });
    }
    let core = spawn(ws, transport, Role::Server, limits, None);
    Ok(ServerChannel {
        core,
        peer,
        peer_version: hello.version,
        verdict: welcome.verdict,
    })
}

fn ceiling_frame(ceiling: &Ceiling) -> Result<Ceiling, CeilingError> {
    let len = Frame::Ceiling(ceiling.clone())
        .encode()
        .map_err(|_| CeilingError::TooLarge)?
        .len();
    if len > MAX_CEILING_FRAME {
        return Err(CeilingError::TooLarge);
    }
    Ok(ceiling.clone())
}

// === The handles ===

struct Core {
    cmd: mpsc::UnboundedSender<Cmd>,
    state: watch::Receiver<Option<Closed>>,
    ceiling: watch::Receiver<Option<Ceiling>>,
}

impl Core {
    async fn closed(&self) -> Closed {
        let mut state = self.state.clone();
        let why = state.wait_for(|s| s.is_some()).await.map(|s| s.clone());
        why.ok().flatten().unwrap_or(Closed::Local)
    }
}

/// The server's end of a channel with a connector.
pub struct ServerChannel {
    core: Core,
    peer: PublicKey,
    peer_version: Version,
    verdict: Verdict,
}

impl ServerChannel {
    /// Asks the connector to open `host:port` for `agent` of `project`; waits for its `opened` or `refused`.
    pub async fn open(
        &self,
        host: &str,
        port: u16,
        agent: &str,
        project: &str,
    ) -> Result<Stream, OpenError> {
        let (reply, answer) = oneshot::channel();
        self.core
            .cmd
            .send(Cmd::Open {
                host: host.into(),
                port,
                agent: agent.into(),
                project: project.into(),
                reply,
            })
            .map_err(|_| OpenError::Closed)?;
        answer.await.unwrap_or(Err(OpenError::Closed))
    }

    /// The connector's key, as pinned.
    pub fn peer_key(&self) -> PublicKey {
        self.peer
    }

    pub fn peer_version(&self) -> Version {
        self.peer_version
    }

    /// `Current` or `Outdated` (« to update »).
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// The connector's ceiling, once it has sent it (at each connection, and again when it reads its file again).
    pub fn ceiling(&self) -> Option<Ceiling> {
        self.core.ceiling.borrow().clone()
    }

    /// To follow the ceiling as the connector sends it again.
    pub fn watch_ceiling(&self) -> watch::Receiver<Option<Ceiling>> {
        self.core.ceiling.clone()
    }

    pub fn is_closed(&self) -> bool {
        self.core.state.borrow().is_some()
    }

    /// Waits for the channel's end, and says why.
    pub async fn closed(&self) -> Closed {
        self.core.closed().await
    }

    /// Closes the channel: every stream is reset.
    pub fn close(&self) {
        let _ = self.core.cmd.send(Cmd::Close);
    }
}

/// The connector's end of a channel with its server.
pub struct ConnectorChannel {
    core: Core,
    incoming: mpsc::UnboundedReceiver<Incoming>,
    welcome: Welcome,
}

impl ConnectorChannel {
    /// The next stream the server asks for; `None` once the channel is closed.
    pub async fn next_open(&mut self) -> Option<Incoming> {
        self.incoming.recv().await
    }

    /// The ceiling again (its file read again).
    pub fn send_ceiling(&self, ceiling: &Ceiling) -> Result<(), CeilingError> {
        let ceiling = ceiling_frame(ceiling)?;
        self.core
            .cmd
            .send(Cmd::Ceiling(ceiling))
            .map_err(|_| CeilingError::Closed)
    }

    /// The server's version and its verdict on this connector's (`Outdated`: to update, [`Welcome::advice`]).
    pub fn welcome(&self) -> Welcome {
        self.welcome
    }

    pub fn is_closed(&self) -> bool {
        self.core.state.borrow().is_some()
    }

    pub async fn closed(&self) -> Closed {
        self.core.closed().await
    }

    pub fn close(&self) {
        let _ = self.core.cmd.send(Cmd::Close);
    }
}

/// A stream the server asks the connector to open: [`accept`](Incoming::accept) it once `host:port` answers, or
/// [`refuse`](Incoming::refuse) it. Dropped unanswered, it is refused (`closing`).
pub struct Incoming {
    open: Open,
    cmd: mpsc::UnboundedSender<Cmd>,
    answered: bool,
}

impl Incoming {
    pub fn open(&self) -> &Open {
        &self.open
    }

    pub fn accept(mut self) -> Stream {
        self.answered = true;
        let (stream, inbound, credit) = new_stream(self.open.id, self.cmd.clone());
        let _ = self.cmd.send(Cmd::Answer {
            id: self.open.id,
            answer: Answer::Accept { inbound, credit },
        });
        stream
    }

    pub fn refuse(mut self, refusal: Refusal) {
        self.answered = true;
        let _ = self.cmd.send(Cmd::Answer {
            id: self.open.id,
            answer: Answer::Refuse(refusal),
        });
    }
}

impl Drop for Incoming {
    fn drop(&mut self) {
        if !self.answered {
            let _ = self.cmd.send(Cmd::Answer {
                id: self.open.id,
                answer: Answer::Refuse(Refusal::new(
                    RefusalCode::Closing,
                    "the connector did not answer",
                )),
            });
        }
    }
}

// === The streams ===

enum Inbound {
    Data(Vec<u8>),
    Fin,
    Reset,
}

/// What a writer may still send: the window the peer granted.
struct Credit(Mutex<CreditState>);

struct CreditState {
    available: u64,
    closed: bool,
    waker: Option<Waker>,
}

impl Credit {
    fn new() -> Arc<Credit> {
        Arc::new(Credit(Mutex::new(CreditState {
            available: INITIAL_WINDOW as u64,
            closed: false,
            waker: None,
        })))
    }

    /// The window after `n` more.
    fn add(&self, n: u32) -> u64 {
        let mut st = self.0.lock().expect("the credit");
        st.available += n as u64;
        if let Some(w) = st.waker.take() {
            w.wake();
        }
        st.available
    }

    fn close(&self) {
        let mut st = self.0.lock().expect("the credit");
        st.closed = true;
        if let Some(w) = st.waker.take() {
            w.wake();
        }
    }
}

/// Tells the channel when both halves of a stream are gone.
struct Guard {
    id: StreamId,
    cmd: mpsc::UnboundedSender<Cmd>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.cmd.send(Cmd::Dropped { id: self.id });
    }
}

fn new_stream(
    id: StreamId,
    cmd: mpsc::UnboundedSender<Cmd>,
) -> (Stream, mpsc::UnboundedSender<Inbound>, Arc<Credit>) {
    let (inbound, rx) = mpsc::unbounded_channel();
    let credit = Credit::new();
    let guard = Arc::new(Guard {
        id,
        cmd: cmd.clone(),
    });
    let stream = Stream {
        read: StreamReader {
            id,
            rx,
            buf: Vec::new(),
            pos: 0,
            done: false,
            cmd: cmd.clone(),
            _guard: guard.clone(),
        },
        write: StreamWriter {
            id,
            credit: credit.clone(),
            cmd,
            fin_sent: false,
            guard,
        },
    };
    (stream, inbound, credit)
}

/// One stream of a channel, both directions. Dropped whole before both ends finished, it is reset (`close`); its
/// writer dropped alone, or shut down, ends its direction (as TCP's FIN).
pub struct Stream {
    // The reader first: dropped first, the writer then knows it is the last.
    read: StreamReader,
    write: StreamWriter,
}

impl Stream {
    pub fn id(&self) -> StreamId {
        self.read.id
    }

    pub fn into_split(self) -> (StreamReader, StreamWriter) {
        (self.read, self.write)
    }
}

pub struct StreamReader {
    id: StreamId,
    rx: mpsc::UnboundedReceiver<Inbound>,
    buf: Vec<u8>,
    pos: usize,
    done: bool,
    cmd: mpsc::UnboundedSender<Cmd>,
    _guard: Arc<Guard>,
}

pub struct StreamWriter {
    id: StreamId,
    credit: Arc<Credit>,
    cmd: mpsc::UnboundedSender<Cmd>,
    fin_sent: bool,
    guard: Arc<Guard>,
}

impl AsyncRead for StreamReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = &mut *self;
        loop {
            if this.pos < this.buf.len() {
                let n = (this.buf.len() - this.pos).min(out.remaining());
                out.put_slice(&this.buf[this.pos..this.pos + n]);
                this.pos += n;
                let _ = this.cmd.send(Cmd::Consumed { id: this.id, n });
                return Poll::Ready(Ok(()));
            }
            if this.done {
                return Poll::Ready(Ok(()));
            }
            match this.rx.poll_recv(cx) {
                Poll::Ready(Some(Inbound::Data(b))) => {
                    this.buf = b;
                    this.pos = 0;
                }
                Poll::Ready(Some(Inbound::Fin)) => this.done = true,
                Poll::Ready(Some(Inbound::Reset)) => {
                    this.done = true;
                    return Poll::Ready(Err(io::ErrorKind::ConnectionReset.into()));
                }
                Poll::Ready(None) => {
                    this.done = true;
                    return Poll::Ready(Err(io::ErrorKind::ConnectionAborted.into()));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl AsyncWrite for StreamWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = &mut *self;
        let mut st = this.credit.0.lock().expect("the credit");
        if st.closed || this.fin_sent {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if st.available == 0 {
            st.waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let n = buf.len().min(DATA_CHUNK).min(st.available as usize);
        st.available -= n as u64;
        drop(st);
        let sent = this.cmd.send(Cmd::Data {
            id: this.id,
            bytes: buf[..n].to_vec(),
        });
        Poll::Ready(match sent {
            Ok(()) => Ok(n),
            Err(_) => Err(io::ErrorKind::BrokenPipe.into()),
        })
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        if !self.fin_sent {
            self.fin_sent = true;
            let _ = self.cmd.send(Cmd::Fin { id: self.id });
        }
        Poll::Ready(Ok(()))
    }
}

impl Drop for StreamWriter {
    fn drop(&mut self) {
        // The reader still there: this direction ends, the other goes on.
        if !self.fin_sent && Arc::strong_count(&self.guard) > 1 {
            let _ = self.cmd.send(Cmd::Fin { id: self.id });
        }
    }
}

impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.read).poll_read(cx, buf)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.write).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.write).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.write).poll_shutdown(cx)
    }
}

// === The task ===

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Server,
    Connector,
}

enum Answer {
    Accept {
        inbound: mpsc::UnboundedSender<Inbound>,
        credit: Arc<Credit>,
    },
    Refuse(Refusal),
}

enum Cmd {
    Open {
        host: String,
        port: u16,
        agent: String,
        project: String,
        reply: oneshot::Sender<Result<Stream, OpenError>>,
    },
    Answer {
        id: StreamId,
        answer: Answer,
    },
    Data {
        id: StreamId,
        bytes: Vec<u8>,
    },
    Fin {
        id: StreamId,
    },
    Consumed {
        id: StreamId,
        n: usize,
    },
    Dropped {
        id: StreamId,
    },
    Ceiling(Ceiling),
    Close,
}

enum Phase {
    /// Asked by the server, its `opened` awaited.
    Asked(oneshot::Sender<Result<Stream, OpenError>>),
    /// Offered to the connector's caller, its answer awaited.
    Offered,
    Live {
        inbound: mpsc::UnboundedSender<Inbound>,
        credit: Arc<Credit>,
    },
}

struct Entry {
    phase: Phase,
    /// What the peer may still send before this side grants more.
    recv_left: u64,
    /// Read by this side's reader, not yet granted again.
    to_grant: u64,
    fin_out: bool,
    fin_in: bool,
    last_active: Instant,
}

impl Entry {
    fn new(phase: Phase, now: Instant) -> Entry {
        Entry {
            phase,
            recv_left: INITIAL_WINDOW as u64,
            to_grant: 0,
            fin_out: false,
            fin_in: false,
            last_active: now,
        }
    }

    /// The stream is gone before its end: its reader is reset, its writer stopped, its opener told.
    fn reset(self) {
        match self.phase {
            Phase::Asked(reply) => {
                let _ = reply.send(Err(OpenError::Closed));
            }
            Phase::Offered => {}
            Phase::Live { inbound, credit } => {
                let _ = inbound.send(Inbound::Reset);
                credit.close();
            }
        }
    }
}

struct Task {
    role: Role,
    limits: Limits,
    transport: Transport,
    out: mpsc::UnboundedSender<Vec<u8>>,
    cmd: mpsc::WeakUnboundedSender<Cmd>,
    streams: HashMap<StreamId, Entry>,
    next_id: StreamId,
    rate: Rate,
    incoming: Option<mpsc::UnboundedSender<Incoming>>,
    ceiling: watch::Sender<Option<Ceiling>>,
    ping_nonce: u64,
}

fn spawn<S>(
    ws: Ws<S>,
    transport: Transport,
    role: Role,
    limits: Limits,
    incoming: Option<mpsc::UnboundedSender<Incoming>>,
) -> Core
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (state_tx, state) = watch::channel(None);
    let (ceiling_tx, ceiling) = watch::channel(None);
    let (out, out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (sink, read) = ws.split();
    let writer = tokio::spawn(write_all(sink, out_rx));
    let task = Task {
        role,
        limits,
        transport,
        out,
        cmd: cmd_tx.downgrade(),
        streams: HashMap::new(),
        next_id: 0,
        rate: Rate::default(),
        incoming,
        ceiling: ceiling_tx,
        ping_nonce: 0,
    };
    tokio::spawn(async move {
        let why = task.run(read, cmd_rx, writer).await;
        let _ = state_tx.send(Some(why));
    });
    Core {
        cmd: cmd_tx,
        state,
        ceiling,
    }
}

/// Sends what the task seals, in order, flushing when nothing more waits; closes the WebSocket at the end.
async fn write_all<Si>(mut sink: Si, mut rx: mpsc::UnboundedReceiver<Vec<u8>>) -> Result<(), String>
where
    Si: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    while let Some(m) = rx.recv().await {
        sink.feed(Message::Binary(m.into()))
            .await
            .map_err(|e| e.to_string())?;
        while let Ok(m) = rx.try_recv() {
            sink.feed(Message::Binary(m.into()))
                .await
                .map_err(|e| e.to_string())?;
        }
        sink.flush().await.map_err(|e| e.to_string())?;
    }
    let _ = sink.close().await;
    Ok(())
}

type Result2 = Result<(), String>;

impl Task {
    async fn run<R>(
        mut self,
        mut read: R,
        mut cmd_rx: mpsc::UnboundedReceiver<Cmd>,
        mut writer: tokio::task::JoinHandle<Result<(), String>>,
    ) -> Closed
    where
        R: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
            + Unpin,
    {
        let start = Instant::now();
        let mut last_heard = start;
        let mut ping = tokio::time::interval_at(start + PING_EVERY, PING_EVERY);
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let every =
            (self.limits.stream_idle / 10).clamp(Duration::from_secs(1), Duration::from_secs(60));
        let mut sweep = tokio::time::interval_at(start + every, every);
        sweep.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let why = loop {
            let dead_at = last_heard + DEAD_AFTER;
            tokio::select! {
                message = read.next() => {
                    last_heard = Instant::now();
                    match message {
                        Some(Ok(Message::Binary(b))) => {
                            if let Err(e) = self.on_message(&b) {
                                break Closed::Protocol(e);
                            }
                        }
                        None | Some(Ok(Message::Close(_))) => break Closed::ByPeer,
                        Some(Ok(Message::Text(_))) => break Closed::Protocol("a text message".into()),
                        Some(Ok(_)) => {}
                        Some(Err(e)) => break Closed::Network(e.to_string()),
                    }
                }
                cmd = cmd_rx.recv() => match cmd {
                    None | Some(Cmd::Close) => break Closed::Local,
                    Some(cmd) => {
                        if let Err(e) = self.on_cmd(cmd) {
                            break Closed::Protocol(e);
                        }
                    }
                },
                _ = ping.tick() => {
                    self.ping_nonce += 1;
                    let nonce = self.ping_nonce;
                    if let Err(e) = self.send(&Frame::Ping { nonce }) {
                        break Closed::Protocol(e);
                    }
                }
                _ = sweep.tick() => {
                    if let Err(e) = self.sweep() {
                        break Closed::Protocol(e);
                    }
                }
                _ = tokio::time::sleep_until(dead_at) => break Closed::Silent,
                done = &mut writer => {
                    break Closed::Network(match done {
                        Ok(Err(e)) => e,
                        _ => "the writer stopped".into(),
                    });
                }
            }
        };
        for (_, entry) in self.streams.drain() {
            entry.reset();
        }
        self.incoming = None;
        why
    }

    fn send(&mut self, frame: &Frame) -> Result2 {
        let plain = frame.encode().map_err(|e| e.to_string())?;
        let sealed = self.transport.seal(&plain).map_err(|e| e.to_string())?;
        // The writer gone, the task learns it from its handle.
        let _ = self.out.send(sealed);
        Ok(())
    }

    fn on_message(&mut self, sealed: &[u8]) -> Result2 {
        if sealed.len() > self.limits.max_frame() + TAG {
            return Err(format!("a frame of {} bytes", sealed.len()));
        }
        let plain = self.transport.open(sealed).map_err(|e| e.to_string())?;
        match Frame::decode(&plain) {
            Ok(frame) => self.on_frame(frame),
            // A frame of a later minor: not for this side.
            Err(DecodeError::UnknownKind(_)) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn on_frame(&mut self, frame: Frame) -> Result2 {
        let now = Instant::now();
        match frame {
            Frame::Open(open) => {
                if self.role != Role::Connector {
                    return Err("an open from the connector".into());
                }
                if open.id == 0 || self.streams.contains_key(&open.id) {
                    return Err(format!("stream {} opened twice", open.id));
                }
                let id = open.id;
                // Every open asked counts in its second, refused or not.
                let admitted = self.rate.admit(now, self.limits.opens_per_second);
                if !admitted || self.streams.len() >= self.limits.max_streams {
                    return self.send(&Frame::Refused {
                        id,
                        refusal: Refusal::new(
                            RefusalCode::Limits,
                            format!(
                                "the connector holds {} streams at most, {} opened a second",
                                self.limits.max_streams, self.limits.opens_per_second
                            ),
                        ),
                    });
                }
                let offered = match (&self.incoming, self.cmd.upgrade()) {
                    (Some(tx), Some(cmd)) => tx
                        .send(Incoming {
                            open,
                            cmd,
                            answered: false,
                        })
                        .map_err(|e| {
                            // Not listened to: refused by its drop, but it is not in the table yet.
                            let mut lost = e.0;
                            lost.answered = true;
                        })
                        .is_ok(),
                    _ => false,
                };
                if offered {
                    self.streams.insert(id, Entry::new(Phase::Offered, now));
                    Ok(())
                } else {
                    self.send(&Frame::Refused {
                        id,
                        refusal: Refusal::new(RefusalCode::Closing, "the connector is closing"),
                    })
                }
            }
            Frame::Opened { id } => {
                if self.role != Role::Server {
                    return Err("an opened from the server".into());
                }
                let Some(entry) = self.streams.get_mut(&id) else {
                    return Ok(());
                };
                if !matches!(entry.phase, Phase::Asked(_)) {
                    return Err(format!("stream {id} opened again"));
                }
                let cmd = self.cmd.upgrade().ok_or("the channel is gone")?;
                let (stream, inbound, credit) = new_stream(id, cmd);
                let Phase::Asked(reply) =
                    std::mem::replace(&mut entry.phase, Phase::Live { inbound, credit })
                else {
                    unreachable!()
                };
                entry.last_active = now;
                // The opener gone, the stream is dropped here and closed by its guard.
                let _ = reply.send(Ok(stream));
                Ok(())
            }
            Frame::Refused { id, refusal } => {
                if self.role != Role::Server {
                    return Err("a refused from the server".into());
                }
                match self.streams.get(&id).map(|e| &e.phase) {
                    None => Ok(()),
                    Some(Phase::Asked(_)) => {
                        if let Some(Entry {
                            phase: Phase::Asked(reply),
                            ..
                        }) = self.streams.remove(&id)
                        {
                            let _ = reply.send(Err(OpenError::Refused(refusal)));
                        }
                        Ok(())
                    }
                    Some(_) => Err(format!("stream {id} refused once open")),
                }
            }
            Frame::Data { id, bytes } => {
                let Some(entry) = self.streams.get_mut(&id) else {
                    return Ok(());
                };
                let Phase::Live { inbound, .. } = &entry.phase else {
                    return Err(format!("data on stream {id} before it is open"));
                };
                if entry.fin_in {
                    return Err(format!("data on stream {id} after its end"));
                }
                entry.last_active = now;
                if bytes.is_empty() {
                    entry.fin_in = true;
                    let _ = inbound.send(Inbound::Fin);
                    if entry.fin_out {
                        self.streams.remove(&id);
                    }
                    return Ok(());
                }
                let len = bytes.len() as u64;
                if len > entry.recv_left {
                    return Err(format!("stream {id} beyond its window"));
                }
                entry.recv_left -= len;
                if inbound.send(Inbound::Data(bytes)).is_err() {
                    // Its reader is gone: read, as far as the peer is concerned.
                    return self.grant(id, len as usize);
                }
                Ok(())
            }
            Frame::Close { id } => {
                if let Some(entry) = self.streams.remove(&id) {
                    entry.reset();
                }
                Ok(())
            }
            Frame::Window { id, increment } => {
                let Some(entry) = self.streams.get_mut(&id) else {
                    return Ok(());
                };
                if let Phase::Live { credit, .. } = &entry.phase {
                    if credit.add(increment) > MAX_WINDOW {
                        return Err(format!("stream {id}'s window beyond 2³¹"));
                    }
                }
                Ok(())
            }
            Frame::Ping { nonce } => self.send(&Frame::Pong { nonce }),
            Frame::Pong { .. } => Ok(()),
            Frame::Ceiling(c) => {
                if self.role != Role::Server {
                    return Err("a ceiling from the server".into());
                }
                self.ceiling.send_replace(Some(c));
                Ok(())
            }
        }
    }

    /// `n` bytes read on `id`: granted again once a quarter of the window waits.
    fn grant(&mut self, id: StreamId, n: usize) -> Result2 {
        let Some(entry) = self.streams.get_mut(&id) else {
            return Ok(());
        };
        if entry.fin_in {
            return Ok(());
        }
        entry.to_grant += n as u64;
        if entry.to_grant < (INITIAL_WINDOW / 4) as u64 {
            return Ok(());
        }
        let increment = entry.to_grant as u32;
        entry.recv_left += entry.to_grant;
        entry.to_grant = 0;
        self.send(&Frame::Window { id, increment })
    }

    fn on_cmd(&mut self, cmd: Cmd) -> Result2 {
        let now = Instant::now();
        match cmd {
            Cmd::Open {
                host,
                port,
                agent,
                project,
                reply,
            } => {
                let admitted = self.rate.admit(now, self.limits.opens_per_second);
                if !admitted || self.streams.len() >= self.limits.max_streams {
                    let _ = reply.send(Err(OpenError::Limits));
                    return Ok(());
                }
                loop {
                    self.next_id = self.next_id.wrapping_add(1).max(1);
                    if !self.streams.contains_key(&self.next_id) {
                        break;
                    }
                }
                let id = self.next_id;
                let frame = Frame::Open(Open {
                    id,
                    host,
                    port,
                    agent,
                    project,
                });
                if frame.encode().is_err() {
                    let _ = reply.send(Err(OpenError::Invalid));
                    return Ok(());
                }
                self.streams
                    .insert(id, Entry::new(Phase::Asked(reply), now));
                self.send(&frame)
            }
            Cmd::Answer { id, answer } => {
                let offered = matches!(
                    self.streams.get(&id).map(|e| &e.phase),
                    Some(Phase::Offered)
                );
                match answer {
                    Answer::Accept { inbound, credit } => {
                        if !offered {
                            // Closed by the server meanwhile.
                            let _ = inbound.send(Inbound::Reset);
                            credit.close();
                            return Ok(());
                        }
                        let entry = self.streams.get_mut(&id).expect("offered");
                        entry.phase = Phase::Live { inbound, credit };
                        entry.last_active = now;
                        self.send(&Frame::Opened { id })
                    }
                    Answer::Refuse(refusal) => {
                        if !offered {
                            return Ok(());
                        }
                        self.streams.remove(&id);
                        self.send(&Frame::Refused { id, refusal })
                    }
                }
            }
            Cmd::Data { id, bytes } => {
                match self.streams.get_mut(&id) {
                    Some(entry) if matches!(entry.phase, Phase::Live { .. }) && !entry.fin_out => {
                        entry.last_active = now;
                    }
                    _ => return Ok(()),
                }
                self.send(&Frame::Data { id, bytes })
            }
            Cmd::Fin { id } => {
                let Some(entry) = self.streams.get_mut(&id) else {
                    return Ok(());
                };
                if !matches!(entry.phase, Phase::Live { .. }) || entry.fin_out {
                    return Ok(());
                }
                entry.fin_out = true;
                entry.last_active = now;
                if entry.fin_in {
                    self.streams.remove(&id);
                }
                self.send(&Frame::Data { id, bytes: vec![] })
            }
            Cmd::Consumed { id, n } => self.grant(id, n),
            Cmd::Dropped { id } => {
                let live = matches!(
                    self.streams.get(&id).map(|e| &e.phase),
                    Some(Phase::Live { .. })
                );
                if !live {
                    return Ok(());
                }
                if let Some(entry) = self.streams.remove(&id) {
                    entry.reset();
                }
                self.send(&Frame::Close { id })
            }
            Cmd::Ceiling(c) => self.send(&Frame::Ceiling(c)),
            Cmd::Close => Ok(()),
        }
    }

    /// The streams silent for `stream_idle` are closed, on both sides.
    fn sweep(&mut self) -> Result2 {
        let now = Instant::now();
        let idle: Vec<StreamId> = self
            .streams
            .iter()
            .filter(|(_, e)| now.duration_since(e.last_active) >= self.limits.stream_idle)
            .map(|(id, _)| *id)
            .collect();
        for id in idle {
            let entry = self.streams.remove(&id).expect("listed");
            let offered = matches!(entry.phase, Phase::Offered);
            entry.reset();
            if offered {
                self.send(&Frame::Refused {
                    id,
                    refusal: Refusal::new(RefusalCode::Closing, "no answer in time"),
                })?;
            } else {
                self.send(&Frame::Close { id })?;
            }
        }
        Ok(())
    }
}
