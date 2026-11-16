//! The WebSocket of the remote API (`docs/REMOTE.md`, « La fondation commune », points 2, 3 and 6).
//!
//! - **Off unless asked**: a build of development of the desktop app serves it when `ESCOUADE_REMOTE_API` gives a
//!   port (never a release: a variable opens nothing there), a process without a window when it asks
//!   (`Headless::serve_remote`); never while the policies of the device forbid it (`servers.remoteApi`), checked at
//!   the start and at each connection.
//! - **On the loopback alone**: it listens on 127.0.0.1, and is reached through a tunnel (SSH, WireGuard: the server
//!   mode's). No TLS here.
//! - **Before the WebSocket opens**: the path, a `Host` of the loopback (DNS rebinding), and no `Origin` but the one of
//!   Escouade's own window (a web page of the machine is refused).
//! - **The handshake**: the first message gives the device's token (compared in constant time, `devices`) and the
//!   client's version, which must be the server's (« Mettre à jour le serveur »).
//! - **Then** the calls, each on the lane of its command (`windowthread::lane_of`, as the IPC: a terminal's
//!   keystrokes keep their order), as the client of this connection (`remoteapi::Client`); the channels and the
//!   events pushed on the same connection. A message past `proto::MAX_MESSAGE` closes it.
//! - **The log** (`app.log`) says each connection, its device's name, each refusal and why: never a token, a path of
//!   the disk nor an argument.

use super::devices::Devices;
use super::proto::{self, Frame, ToClient, ToServer};
use super::{codes, Call, Channels, Client, Failure, Outcome, Reply, Services};
use crate::windowthread::{lane_of, Lanes, Running};
use anyhow::{Context, Result};
use futures_util::{FutureExt, SinkExt, StreamExt};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::ipc::InvokeResponseBody;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Notify};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message;

/// The variable that turns the API on in a build of development of the desktop app: the port to listen on (0: any
/// free one).
#[cfg(debug_assertions)]
pub const PORT_VAR: &str = "ESCOUADE_REMOTE_API";
/// In a debug build only, a device token the tests give (fondation 2b's e2e): a device « test » kept in memory.
#[cfg(debug_assertions)]
pub const TEST_TOKEN_VAR: &str = "ESCOUADE_REMOTE_API_TOKEN";

/// A connection that passed the handshake.
struct Conn {
    client: Client,
    device: String,
    tx: mpsc::UnboundedSender<Message>,
    lanes: Arc<Lanes>,
    /// Told to end: its device revoked, the server stopped.
    kicked: Notify,
}

impl Conn {
    fn text(&self, text: String) {
        let _ = self.tx.send(Message::text(text));
    }

    fn send(&self, message: &ToClient) {
        match serde_json::to_string(message) {
            Ok(text) => self.text(text),
            Err(e) => log::warn!("remote API: a message not written: {e}"),
        }
    }

    fn answer(&self, id: u64, out: Outcome) {
        match out {
            Ok(InvokeResponseBody::Json(json)) => self.text(proto::reply_ok(id, &json)),
            Ok(InvokeResponseBody::Raw(bytes)) => {
                let _ = self
                    .tx
                    .send(Message::binary(proto::frame(Frame::Reply, id, &bytes)));
            }
            Err(Failure::Command(error)) => self.send(&ToClient::Reply {
                id,
                ok: None,
                error: Some(error),
                code: None,
            }),
            Err(Failure::Refused { code, message }) => self.send(&ToClient::Reply {
                id,
                ok: None,
                error: Some(serde_json::Value::String(message)),
                code: Some(code.to_string()),
            }),
        }
    }
}

/// The channels of a connection: what they are sent goes to its client, tagged with their id.
struct ConnChannels(Arc<Conn>);

impl Channels for ConnChannels {
    fn open(&self, id: u32) -> Box<dyn Fn(InvokeResponseBody) -> tauri::Result<()> + Send + Sync> {
        let conn = Arc::downgrade(&self.0);
        Box::new(move |body| {
            // A client gone: what its channels are sent goes nowhere (its terminals run on).
            let Some(conn) = conn.upgrade() else {
                return Ok(());
            };
            match body {
                InvokeResponseBody::Json(json) => conn.text(proto::channel_json(id, &json)),
                InvokeResponseBody::Raw(bytes) => {
                    let _ = conn.tx.send(Message::binary(proto::frame(
                        Frame::Channel,
                        u64::from(id),
                        &bytes,
                    )));
                }
            }
            Ok(())
        })
    }
}

/// The remote API, served.
pub struct RemoteApi {
    services: Arc<Services>,
    devices: Devices,
    addr: SocketAddr,
    next: AtomicU64,
    live: Mutex<HashMap<u64, Arc<Conn>>>,
    stopped: Notify,
}

impl RemoteApi {
    /// The API served on 127.0.0.1:`port` (0: a free port, `addr` says which), for the core of `services`; refused
    /// when the policies of the device forbid it. On a runtime of Tokio.
    pub async fn start(services: Arc<Services>, port: u16) -> Result<Arc<Self>> {
        let core = services.core().clone();
        core.locks.read().check_remote_api()?;
        #[cfg(debug_assertions)]
        let devices = match std::env::var(TEST_TOKEN_VAR) {
            Ok(token) if !token.is_empty() => Devices::of_test("test", token, core.secrets.clone()),
            _ => Devices::load(&core.data, core.secrets.clone()),
        };
        #[cfg(not(debug_assertions))]
        let devices = Devices::load(&core.data, core.secrets.clone());
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .await
            .with_context(|| format!("remote API: port {port}"))?;
        let addr = listener.local_addr()?;
        let api = Arc::new(Self {
            services,
            devices,
            addr,
            next: AtomicU64::new(1),
            live: Mutex::new(HashMap::new()),
            stopped: Notify::new(),
        });
        log::info!("remote API: listening on {addr}");
        let me = api.clone();
        tokio::spawn(async move { me.accept(listener).await });
        Ok(api)
    }

    /// Where it listens: 127.0.0.1 and its port.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The devices that may connect.
    pub fn devices(&self) -> &Devices {
        &self.devices
    }

    /// How many clients are connected.
    pub fn clients(&self) -> usize {
        self.live.lock().len()
    }

    /// The device `id` revoked: its token forgotten, its connections closed.
    pub fn revoke(&self, id: &str) -> Result<bool> {
        let gone = self.devices.revoke(id)?;
        for conn in self.live.lock().values().filter(|c| c.device == id) {
            conn.kicked.notify_one();
        }
        if gone {
            log::info!("remote API: device {id} revoked");
        }
        Ok(gone)
    }

    /// No more connections, and the ones open closed.
    pub fn stop(&self) {
        self.stopped.notify_one();
        for conn in self.live.lock().values() {
            conn.kicked.notify_one();
        }
    }

    async fn accept(self: Arc<Self>, listener: TcpListener) {
        loop {
            tokio::select! {
                _ = self.stopped.notified() => break,
                accepted = listener.accept() => match accepted {
                    Ok((stream, _)) => {
                        let me = self.clone();
                        tokio::spawn(async move { me.connection(stream).await });
                    }
                    Err(e) => {
                        log::warn!("remote API: accept: {e}");
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                },
            }
        }
        log::info!("remote API: stopped");
    }

    async fn connection(self: Arc<Self>, stream: TcpStream) {
        let config = WebSocketConfig::default()
            .max_message_size(Some(proto::MAX_MESSAGE))
            .max_frame_size(Some(proto::MAX_MESSAGE));
        // The callback's type is tungstenite's: its error, an HTTP answer, is large.
        #[allow(clippy::result_large_err)]
        let guard = |req: &Request, resp: Response| match guard(req) {
            Ok(()) => Ok(resp),
            Err((status, why)) => {
                log::warn!("remote API: refused a connection: {why}");
                let mut refused = ErrorResponse::new(Some(why));
                *refused.status_mut() = status;
                Err(refused)
            }
        };
        let ws = match tokio_tungstenite::accept_hdr_async_with_config(stream, guard, Some(config))
            .await
        {
            Ok(ws) => ws,
            Err(e) => {
                log::debug!("remote API: handshake of the WebSocket: {e}");
                return;
            }
        };
        let (mut sink, mut stream) = ws.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        let writer = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                let closing = matches!(message, Message::Close(_));
                if sink.send(message).await.is_err() || closing {
                    break;
                }
            }
            let _ = sink.close().await;
        });
        let refuse = |code: &'static str,
                      error: String,
                      server_version: Option<String>,
                      update: Option<String>| {
            log::warn!("remote API: refused a connection ({code}): {error}");
            if let Ok(text) = serde_json::to_string(&ToClient::Refused {
                code: code.into(),
                error: error.clone(),
                server_version,
                update,
            }) {
                let _ = tx.send(Message::text(text));
            }
            let _ = tx.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Policy,
                reason: code.into(),
            })));
        };

        // The handshake: the first message, in time.
        let hello = tokio::time::timeout(proto::HELLO_WAIT, stream.next()).await;
        let (version, token) = match hello {
            Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ToServer>(&text) {
                Ok(ToServer::Hello { version, token }) => (version, token),
                _ => {
                    refuse(codes::HANDSHAKE, handshake_text(), None, None);
                    drop(tx);
                    let _ = writer.await;
                    return;
                }
            },
            _ => {
                refuse(codes::HANDSHAKE, handshake_text(), None, None);
                drop(tx);
                let _ = writer.await;
                return;
            }
        };
        let core = self.services.core().clone();
        let server_version = core.host.version();
        let device = if let Err(e) = core.locks.read().check_remote_api() {
            refuse(codes::POLICY, e.to_string(), None, None);
            None
        } else if let Some(device) = self.devices.find(&token) {
            if version != server_version {
                let update = proto::to_update(&server_version, &version);
                refuse(
                    codes::VERSION,
                    version_text(&server_version, &version, update),
                    Some(server_version.clone()),
                    Some(update.to_string()),
                );
                None
            } else {
                Some(device)
            }
        } else {
            refuse(
                codes::TOKEN,
                tr!(
                    "Jeton d’appareil absent, inconnu ou révoqué.",
                    "Device token missing, unknown or revoked."
                ),
                None,
                None,
            );
            None
        };
        let Some(device) = device else {
            drop(tx);
            let _ = writer.await;
            return;
        };

        let client = Client::Remote(self.next.fetch_add(1, Ordering::Relaxed));
        let conn = Arc::new(Conn {
            client: client.clone(),
            device: device.id.clone(),
            tx,
            lanes: Lanes::new(Arc::new(Running::default())),
            kicked: Notify::new(),
        });
        self.live.lock().insert(
            match client {
                Client::Remote(n) => n,
                Client::Window => 0,
            },
            conn.clone(),
        );
        self.devices.seen(&device.id);
        log::info!(
            "remote API: device « {} » connected ({})",
            device.name,
            client.label()
        );
        conn.send(&ToClient::Welcome {
            version: server_version,
            client: client.label(),
            device: device.name.clone(),
        });
        // What the host publishes (a process without a window), pushed as it comes.
        let host_events = core.host.events().map(|mut events| {
            let conn = Arc::downgrade(&conn);
            tokio::spawn(async move {
                while let Some(event) = events.recv().await {
                    let Some(conn) = conn.upgrade() else { break };
                    match serde_json::to_value(&event) {
                        Ok(event) => conn.send(&ToClient::Host { event }),
                        Err(e) => log::warn!("remote API: a host event not written: {e}"),
                    }
                }
            })
        });

        loop {
            tokio::select! {
                _ = conn.kicked.notified() => break,
                message = stream.next() => match message {
                    None | Some(Ok(Message::Close(_))) => break,
                    Some(Err(e)) => {
                        log::warn!("remote API: {} closed: {e}", client.label());
                        break;
                    }
                    Some(Ok(Message::Text(text))) => match serde_json::from_str::<ToServer>(&text) {
                        Ok(ToServer::Call { id, cmd, args }) => self.dispatch(&conn, id, &cmd, args),
                        _ => conn.send(&ToClient::Error {
                            code: codes::BAD_MESSAGE.into(),
                            error: "not a call of the remote API".into(),
                        }),
                    },
                    Some(Ok(Message::Binary(_))) => conn.send(&ToClient::Error {
                        code: codes::BAD_MESSAGE.into(),
                        error: "a client's binary messages are not read yet".into(),
                    }),
                    Some(Ok(_)) => {}
                },
            }
        }

        if let Some(task) = host_events {
            task.abort();
        }
        self.live.lock().retain(|_, c| !Arc::ptr_eq(c, &conn));
        core.forget_client(&client);
        let _ = conn.tx.send(Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: "".into(),
        })));
        log::info!(
            "remote API: device « {} » disconnected ({})",
            device.name,
            client.label()
        );
        drop(conn);
        let _ = writer.await;
    }

    /// The call `id` of the command `name`, run as the IPC runs it, on its lane; its answer sent back.
    fn dispatch(
        &self,
        conn: &Arc<Conn>,
        id: u64,
        name: &str,
        args: serde_json::Map<String, serde_json::Value>,
    ) {
        let Some(command) = super::find(name) else {
            return conn.answer(
                id,
                Err(Failure::refused(
                    codes::UNKNOWN_COMMAND,
                    format!("Command {name} not found"),
                )),
            );
        };
        if !command.class.served() {
            log::info!(
                "remote API: {} asked {name}, refused ({})",
                conn.client.label(),
                match command.class {
                    super::Class::Desk => "the desktop's",
                    _ => "not served yet",
                }
            );
            return conn.answer(id, Err(refusal(command)));
        }
        let call = Call {
            command: command.name,
            args,
            services: self.services.clone(),
            channels: Arc::new(ConnChannels(conn.clone())),
        };
        let reply = (command.call)(call);
        let (done, client) = (conn.clone(), conn.client.clone());
        let name = command.name;
        let panicked = move || {
            Err(Failure::refused(
                codes::PANICKED,
                format!("{name}: the command panicked"),
            ))
        };
        match reply {
            Reply::Refused(why) => conn.answer(
                id,
                Err(Failure::refused(
                    codes::UNAVAILABLE,
                    format!("{name} is not served at a distance ({why})"),
                )),
            ),
            Reply::Blocking(job) => conn.lanes.submit(
                lane_of(name),
                Box::new(move || {
                    let out = std::panic::catch_unwind(AssertUnwindSafe(|| {
                        super::as_client(client, job)
                    }))
                    .unwrap_or_else(|_| panicked());
                    done.answer(id, out);
                }),
            ),
            Reply::Future(future) => conn.lanes.submit(
                lane_of(name),
                Box::new(move || {
                    // Started in its turn on its lane, as the IPC starts it; it runs on the runtime.
                    tokio::runtime::Handle::current().spawn(async move {
                        let out = AssertUnwindSafe(super::scoped(client, future))
                            .catch_unwind()
                            .await
                            .unwrap_or_else(|_| panicked());
                        done.answer(id, out);
                    });
                }),
            ),
        }
    }
}

/// Why the command `command`, not served, is refused.
fn refusal(command: &super::Command) -> Failure {
    let name = command.name;
    match command.class {
        super::Class::Desk => Failure::refused(
            codes::DESK_ONLY,
            tr!(
                "« {name} » est une commande du poste : elle ne se sert pas à distance.",
                "“{name}” is a command of the desktop: it is not served at a distance."
            ),
        ),
        _ => Failure::refused(
            codes::NOT_SERVED_YET,
            tr!(
                "« {name} » change de machine en mode serveur : pas encore servie à distance.",
                "“{name}” changes machines in server mode: not served at a distance yet."
            ),
        ),
    }
}

fn handshake_text() -> String {
    tr!(
        "Le premier message doit être la poignée de main (version et jeton), dans les {} s.",
        "The first message must be the handshake (version and token), within {} s.",
        proto::HELLO_WAIT.as_secs()
    )
}

/// « Le serveur est en 2.3.0, ce client en 2.2.1 : mettre à jour le client. »
fn version_text(server: &str, client: &str, update: &str) -> String {
    if update == "client" {
        tr!(
            "Le serveur est en {server}, ce client en {client} : mettre à jour le client.",
            "The server runs {server}, this client {client}: update the client."
        )
    } else {
        tr!(
            "Le serveur est en {server}, ce client en {client} : mettre à jour le serveur.",
            "The server runs {server}, this client {client}: update the server."
        )
    }
}

/// The request that opens the WebSocket, before it opens: its path, a `Host` of the loopback, no `Origin` but
/// Escouade's own window's. Else its HTTP status and why (shown in the log: no secret there).
fn guard(req: &Request) -> std::result::Result<(), (tauri::http::StatusCode, String)> {
    use tauri::http::StatusCode;
    if req.uri().path() != proto::PATH {
        return Err((StatusCode::NOT_FOUND, format!("path {}", req.uri().path())));
    }
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if !loopback_host(host) {
        return Err((
            StatusCode::FORBIDDEN,
            format!("a Host that is not the loopback ({})", shown(host)),
        ));
    }
    if let Some(origin) = req.headers().get("origin") {
        let origin = origin.to_str().unwrap_or("");
        if !proto::ORIGINS.contains(&origin) {
            return Err((
                StatusCode::FORBIDDEN,
                format!("the Origin of a web page ({})", shown(origin)),
            ));
        }
    }
    Ok(())
}

/// `127.0.0.1`, `localhost` or `[::1]`, with any port: through a tunnel, the client's port is its own.
fn loopback_host(host: &str) -> bool {
    let name = if let Some(rest) = host.strip_prefix('[') {
        rest.split(']')
            .next()
            .map(|h| format!("[{h}]"))
            .unwrap_or_default()
    } else {
        host.split(':').next().unwrap_or("").to_string()
    };
    matches!(
        name.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "[::1]"
    )
}

/// A header as the log may show it: 80 characters at most, printable ones.
fn shown(value: &str) -> String {
    value.chars().filter(|c| !c.is_control()).take(80).collect()
}
