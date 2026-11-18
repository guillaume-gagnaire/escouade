//! The remote API of the core (ESC-478, `docs/REMOTE.md`, « La fondation commune », points 2, 3 and 6): the same
//! commands as Tauri's IPC, served over a WebSocket on the loopback, to a client that runs elsewhere (the desktop app
//! of another machine through a tunnel, then the phone).
//!
//! - **One list** (`list.rs`): each command is registered once, by `escouade_command::commands!`, for the IPC and for
//!   this API, with its class ([`Class`]): served as it is, the desktop's only (refused here with a named error), or one
//!   that changes machines in server mode (served when it works as it is, else refused until the server mode does it).
//! - **The call** of a command for this API ([`Call`] → [`Reply`]): `#[tauri::command]` in `commands.rs` is
//!   `escouade_command::command`, which writes `__remote_<name>` beside each command. Its arguments are read as the
//!   IPC reads them (`arg`), its result answered as the IPC answers (`out`).
//! - **The states** a command takes are this module's [`State`] (`commands.rs` uses it in place of Tauri's): read from
//!   Tauri's states for the IPC, from the call's [`Services`] for this API.
//! - **The client** a call comes from ([`Client`]): the window, or a connection of the API. The hub's subscribers and
//!   the interface's state (`UiState`) are kept by client.
//! - **The server** (`server.rs`): the WebSocket, the handshake (versions, device token), its guards.
//! - **The pairing** (`pairing.rs`, ESC-514): a new device's code, exchanged for its token before any handshake.

pub mod devices;
pub mod list;
pub mod outbox;
pub mod pairing;
pub mod proto;
pub mod server;
pub mod sessions;
#[cfg(feature = "desktop")]
pub mod window;

use crate::core::Core;
use serde::Serialize;
use serde_json::Value;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::ipc::InvokeResponseBody;
#[cfg(feature = "desktop")]
use tauri::ipc::{CommandArg, CommandItem, InvokeError};
#[cfg(feature = "desktop")]
use tauri::{Manager, Runtime};

/// What the remote API does of a command (`docs/REMOTE.md`, « Le classement des commandes »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Class {
    /// Served as it is: it acts on the core, whichever machine the client is on.
    Served,
    /// The desktop's only (the window, the tray, the dialogs, the clipboard, the microphone, the global shortcut,
    /// the client's own updater): refused with the error `desk-only`.
    Desk,
    /// It changes machines in server mode (`docs/REMOTE.md`, « Ce qui change de machine »): `served` when it works as
    /// it is, else refused with the error `not-served-yet` until the server mode does it.
    Moves { served: bool },
}

impl Class {
    pub fn served(self) -> bool {
        matches!(self, Class::Served | Class::Moves { served: true })
    }
}

/// A command of the list.
pub struct Command {
    pub name: &'static str,
    pub class: Class,
    /// `__remote_<name>`, written by `#[tauri::command]` (`escouade_command::command`).
    pub call: fn(Call) -> Reply,
}

/// The call of a command the core without a window refuses by its class (the desktop's, those not served yet): its
/// function is not built there (docs/CRATES.md, § 14), the server refuses it before any call.
#[cfg(not(feature = "desktop"))]
pub fn refused_here(_call: Call) -> Reply {
    Reply::refused("not in this process")
}

/// The command `name` of the list.
pub fn find(name: &str) -> Option<&'static Command> {
    list::COMMANDS.iter().find(|c| c.name == name)
}

// ---------- the client of a call ----------

/// Who a call comes from: the window of the desktop app, or a connection of the remote API.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Client {
    Window,
    /// The connection of this number (`server`).
    Remote(u64),
}

impl Client {
    /// As the log and the client are told it: `window`, `c3`.
    pub fn label(&self) -> String {
        match self {
            Client::Window => "window".into(),
            Client::Remote(n) => format!("c{n}"),
        }
    }
}

tokio::task_local! {
    static CLIENT: Client;
}

/// The client of the call running now: the window when no connection of the API called (the IPC, the core's own
/// tasks).
pub fn current() -> Client {
    CLIENT.try_with(Client::clone).unwrap_or(Client::Window)
}

/// `f` run as a call of `client`.
pub fn as_client<T>(client: Client, f: impl FnOnce() -> T) -> T {
    CLIENT.sync_scope(client, f)
}

/// The future `f` run as a call of `client`.
pub fn scoped<F: Future>(client: Client, f: F) -> impl Future<Output = F::Output> {
    CLIENT.scope(client, f)
}

// ---------- what a call is given ----------

/// What the commands find in a process: the core, the states of the desktop app when it is one, and any other state
/// put there (`with`).
pub struct Services {
    core: Arc<Core>,
    #[cfg(feature = "desktop")]
    app: Option<tauri::AppHandle>,
    more: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Services {
    /// The core alone (a process without a window).
    pub fn new(core: Arc<Core>) -> Self {
        Self {
            core,
            #[cfg(feature = "desktop")]
            app: None,
            more: HashMap::new(),
        }
    }

    /// The core and the desktop app's states (`app.manage`).
    #[cfg(feature = "desktop")]
    pub fn of_app(core: Arc<Core>, app: tauri::AppHandle) -> Self {
        Self {
            app: Some(app),
            ..Self::new(core)
        }
    }

    /// With `state` too.
    pub fn with<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        self.more.insert(TypeId::of::<T>(), Box::new(state));
        self
    }

    pub fn core(&self) -> &Arc<Core> {
        &self.core
    }

    /// The state of type `T`, if this process has one.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        if let Some(core) = (&self.core as &dyn Any).downcast_ref::<T>() {
            return Some(core);
        }
        if let Some(state) = self.more.get(&TypeId::of::<T>()) {
            return state.downcast_ref::<T>();
        }
        #[cfg(feature = "desktop")]
        let found = self
            .app
            .as_ref()
            .and_then(|app| app.try_state::<T>())
            .map(|s| s.inner());
        #[cfg(not(feature = "desktop"))]
        let found = None;
        found
    }

    #[cfg(feature = "desktop")]
    pub fn app(&self) -> Option<&tauri::AppHandle> {
        self.app.as_ref()
    }
}

/// Makes the channel of the id a client gave (`__CHANNEL__:<id>`, as the IPC's): what it is sent goes to that client,
/// tagged with the id (`server`).
pub trait Channels: Send + Sync {
    fn open(&self, id: u32) -> Box<dyn Fn(InvokeResponseBody) -> tauri::Result<()> + Send + Sync>;
    /// The channel `id` as a terminal's output goes to it (`TermChannel`): numbered, so that the client resumes it after
    /// a cut (`server`). By default, the bytes alone.
    fn term(&self, id: u32) -> Box<dyn crate::termlog::Sink> {
        Box::new(IpcSink(tauri::ipc::Channel::new(self.open(id))))
    }
}

/// A call of a command by the remote API.
pub struct Call {
    pub command: &'static str,
    /// Its arguments, by the IPC's names (camelCase).
    pub args: serde_json::Map<String, Value>,
    pub services: Arc<Services>,
    pub channels: Arc<dyn Channels>,
    /// Its raw body (`attach_chunk`, `fs_write_bytes`) and the headers that go with it, when it has one.
    pub body: Option<RawBody>,
}

/// The raw body of a call and its headers, as the IPC gives them (`invoke(cmd, bytes, { headers })`).
#[derive(Debug, Default, Clone)]
pub struct RawBody {
    pub bytes: Vec<u8>,
    pub headers: tauri::http::HeaderMap,
}

// ---------- a terminal's output ----------

/// Where a terminal's output goes (`term_spawn`, `run_start`, `test_run_start`, `account_login`, `agent_terminal`):
/// the window's channel (Tauri's IPC: the bytes alone, as before), or a channel of a client of the remote API (numbered:
/// the client resumes it after a cut, `termlog`). The commands take it in place of a `Channel`.
pub enum TermChannel {
    Window(tauri::ipc::Channel<InvokeResponseBody>),
    Remote(Box<dyn crate::termlog::Sink>),
}

impl TermChannel {
    pub fn sink(self) -> Box<dyn crate::termlog::Sink> {
        match self {
            TermChannel::Window(channel) => Box::new(IpcSink(channel)),
            TermChannel::Remote(sink) => sink,
        }
    }
}

/// The window's channel of a terminal: the bytes alone, numbered or not.
struct IpcSink(tauri::ipc::Channel<InvokeResponseBody>);

impl crate::termlog::Sink for IpcSink {
    fn send(&self, _end: u64, data: &[u8]) {
        let _ = self.0.send(InvokeResponseBody::Raw(data.to_vec()));
    }
    fn replay(&self, data: &[u8]) {
        let _ = self.0.send(InvokeResponseBody::Raw(data.to_vec()));
    }
}

#[cfg(feature = "desktop")]
impl<'de, R: Runtime> CommandArg<'de, R> for TermChannel {
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        Ok(TermChannel::Window(CommandArg::from_command(command)?))
    }
}

impl<'c> FromCall<'c> for TermChannel {
    fn from_call(call: &'c Call, key: &str) -> Result<Self, Failure> {
        Ok(TermChannel::Remote(
            call.channels.term(channel_id(call, key)?),
        ))
    }
}

/// The id of the channel the client gave for `key`: `__CHANNEL__:<id>`, as the IPC's.
fn channel_id(call: &Call, key: &str) -> Result<u32, Failure> {
    let value = call.args.get(key);
    value
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("__CHANNEL__:"))
        .and_then(|n| n.parse::<u32>().ok())
        .or_else(|| {
            value
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
        })
        .ok_or_else(|| {
            Failure::Command(Value::String(format!(
                "invalid args `{key}` for command `{}`: expected a string in the `__CHANNEL__:ID` format",
                call.command
            )))
        })
}

// ---------- a raw body ----------

/// The raw body of a call (the chunks of an attachment, the bytes of a file) and its headers, as the commands read
/// it: from Tauri's request for the IPC, from the call's binary frames for the remote API (`server`).
pub enum Body<'a> {
    #[cfg(feature = "desktop")]
    Ipc(tauri::ipc::Request<'a>),
    Remote(Option<&'a RawBody>),
}

impl Body<'_> {
    /// Its bytes, when they came raw (not as JSON).
    pub fn raw(&self) -> Option<&[u8]> {
        match self {
            #[cfg(feature = "desktop")]
            Body::Ipc(request) => match request.body() {
                tauri::ipc::InvokeBody::Raw(bytes) => Some(bytes.as_slice()),
                _ => None,
            },
            Body::Remote(body) => body.map(|b| b.bytes.as_slice()),
        }
    }

    /// The header `name`, as text.
    pub fn header(&self, name: &str) -> Option<&str> {
        let headers = match self {
            #[cfg(feature = "desktop")]
            Body::Ipc(request) => request.headers(),
            Body::Remote(body) => &(*body)?.headers,
        };
        headers.get(name).and_then(|h| h.to_str().ok())
    }
}

#[cfg(feature = "desktop")]
impl<'a, R: Runtime> CommandArg<'a, R> for Body<'a> {
    fn from_command(command: CommandItem<'a, R>) -> Result<Self, InvokeError> {
        Ok(Body::Ipc(CommandArg::from_command(command)?))
    }
}

impl<'c> FromCall<'c> for Body<'c> {
    fn from_call(call: &'c Call, _key: &str) -> Result<Self, Failure> {
        Ok(Body::Remote(call.body.as_ref()))
    }
}

/// What a command answers: its result as the IPC gives it (JSON, or bytes for a `tauri::ipc::Response`), or why it
/// failed.
pub type Outcome = Result<InvokeResponseBody, Failure>;

/// Why a call failed.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    /// The command's own error, as the IPC gives it: its `Err`, serialized (a text for most), or the IPC's text for
    /// arguments it cannot read.
    Command(Value),
    /// Refused by the remote API, with a name the client can act on (`code`).
    Refused { code: &'static str, message: String },
}

impl Failure {
    pub fn refused(code: &'static str, message: String) -> Self {
        Failure::Refused { code, message }
    }
}

/// The machine this process runs on (ESC-537): its name on its network and the account it runs as, what a client
/// says of a server (« escouade@vps.example.net ») and what a server is told of a client (« DESKTOP-7QK2 »).
pub fn this_machine() -> proto::Machine {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let host = env("COMPUTERNAME")
        .or_else(host_name)
        .or_else(|| env("HOSTNAME"))
        .unwrap_or_default();
    let user = env("USER")
        .or_else(|| env("USERNAME"))
        .or_else(|| env("LOGNAME"))
        .unwrap_or_default();
    proto::Machine {
        host: host.trim().to_string(),
        user: user.trim().to_string(),
    }
}

#[cfg(unix)]
fn host_name() -> Option<String> {
    let mut buf = [0u8; 256];
    // SAFETY: the buffer is ours, its length given; gethostname writes a string ended by a 0 within it.
    let ok = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) } == 0;
    if !ok {
        return None;
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    Some(String::from_utf8_lossy(&buf[..end]).into_owned()).filter(|h| !h.is_empty())
}

#[cfg(not(unix))]
fn host_name() -> Option<String> {
    None
}

/// The names of the remote API's refusals: what a client reads to say what to do.
pub mod codes {
    /// The desktop's only command.
    pub const DESK_ONLY: &str = "desk-only";
    /// A command that changes machines, not served yet.
    pub const NOT_SERVED_YET: &str = "not-served-yet";
    /// No such command.
    pub const UNKNOWN_COMMAND: &str = "unknown-command";
    /// What the command needs is not in this process (a state of the desktop app, a request's raw body).
    pub const UNAVAILABLE: &str = "unavailable";
    /// The command panicked.
    pub const PANICKED: &str = "panicked";
    /// The message is not one of the protocol's.
    pub const BAD_MESSAGE: &str = "bad-message";
    /// The client and the server are not of the same version: « Mettre à jour le serveur ».
    pub const VERSION: &str = "version";
    /// No device token, or one that is not (or no longer) a device's.
    pub const TOKEN: &str = "token";
    /// The policies of this device forbid the remote API (`servers.remoteApi`).
    pub const POLICY: &str = "policy";
    /// The first message was not the handshake, or came too late.
    pub const HANDSHAKE: &str = "handshake";
    /// A pairing code refused (`pairing`): none waits, it expired, it is wrong, or it took too many tries.
    pub const PAIRING: &str = "pairing";
    /// Too many connections of this device, or in all.
    pub const TOO_MANY: &str = "too-many-connections";
    /// Too many calls of this connection wait for their answer.
    pub const BUSY: &str = "busy";
    /// A raw body past its bounds (`proto::MAX_BODY`), or not the length its call says.
    pub const TOO_LARGE: &str = "too-large";
    /// The connection read nothing for too long, or let too much wait unread: closed (`proto::CLOSE_SLOW`).
    pub const SLOW: &str = "slow-client";
}

type Job = Box<dyn FnOnce() -> Outcome + Send>;
type Fut = Pin<Box<dyn Future<Output = Outcome> + Send>>;

/// How a command runs for the remote API: a function of its own on a thread of the blocking pool, or its future.
pub enum Reply {
    Blocking(Job),
    Future(Fut),
    Refused(&'static str),
}

impl Reply {
    pub fn blocking(f: impl FnOnce() -> Outcome + Send + 'static) -> Self {
        Reply::Blocking(Box::new(f))
    }

    pub fn future(f: impl Future<Output = Outcome> + Send + 'static) -> Self {
        Reply::Future(Box::pin(f))
    }

    /// A command whose form the remote API cannot call (generic over Tauri's runtime…): `why`.
    pub fn refused(why: &'static str) -> Self {
        Reply::Refused(why)
    }
}

// ---------- the states of the commands ----------

/// A state of the process, as a command takes it: `commands.rs` uses it in place of Tauri's `State`, which only Tauri
/// can make. The same `Deref` and `inner`; read from Tauri's states for the IPC, from the call's [`Services`] for the
/// remote API.
pub struct State<'r, T: Send + Sync + 'static>(&'r T);

impl<'r, T: Send + Sync + 'static> State<'r, T> {
    pub fn inner(&self) -> &'r T {
        self.0
    }
}

impl<T: Send + Sync + 'static> std::ops::Deref for State<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

impl<T: Send + Sync + 'static> Clone for State<'_, T> {
    fn clone(&self) -> Self {
        State(self.0)
    }
}

#[cfg(feature = "desktop")]
impl<'r, 'de: 'r, T: Send + Sync + 'static, R: Runtime> CommandArg<'de, R> for State<'r, T> {
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        let state: tauri::State<'r, T> = CommandArg::from_command(command)?;
        Ok(State(state.inner()))
    }
}

/// A state the process may not have (the voice, the updater, in a process without a window): the command does
/// without.
pub struct Maybe<'r, T: Send + Sync + 'static>(Option<&'r T>);

impl<'r, T: Send + Sync + 'static> Maybe<'r, T> {
    pub fn get(&self) -> Option<&'r T> {
        self.0
    }
}

#[cfg(feature = "desktop")]
impl<'r, 'de: 'r, T: Send + Sync + 'static, R: Runtime> CommandArg<'de, R> for Maybe<'r, T> {
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        Ok(Maybe(
            command
                .message
                .webview_ref()
                .try_state::<T>()
                .map(|s| s.inner()),
        ))
    }
}

/// What the remote API reads a command's arguments with: one of the IPC's names, from the call (`arg`).
pub trait FromCall<'c>: Sized {
    fn from_call(call: &'c Call, key: &str) -> Result<Self, Failure>;
}

impl<'c, T: Send + Sync + 'static> FromCall<'c> for State<'c, T> {
    fn from_call(call: &'c Call, key: &str) -> Result<Self, Failure> {
        call.services.get::<T>().map(State).ok_or_else(|| {
            Failure::refused(
                codes::UNAVAILABLE,
                format!(
                    "{}: `{key}` is not in this process ({})",
                    call.command,
                    std::any::type_name::<T>()
                ),
            )
        })
    }
}

impl<'c, T: Send + Sync + 'static> FromCall<'c> for Maybe<'c, T> {
    fn from_call(call: &'c Call, _key: &str) -> Result<Self, Failure> {
        Ok(Maybe(call.services.get::<T>()))
    }
}

/// A channel the client opened: `__CHANNEL__:<id>`, as the IPC's.
impl<'c, T> FromCall<'c> for tauri::ipc::Channel<T> {
    fn from_call(call: &'c Call, key: &str) -> Result<Self, Failure> {
        Ok(tauri::ipc::Channel::new(
            call.channels.open(channel_id(call, key)?),
        ))
    }
}

/// The desktop app's handle: in the desktop app only.
#[cfg(feature = "desktop")]
impl<'c> FromCall<'c> for tauri::AppHandle {
    fn from_call(call: &'c Call, key: &str) -> Result<Self, Failure> {
        call.services.app().cloned().ok_or_else(|| {
            Failure::refused(
                codes::UNAVAILABLE,
                format!("{}: `{key}` needs the desktop app", call.command),
            )
        })
    }
}

/// Tauri's request: the commands take [`Body`] instead, which both read.
#[cfg(feature = "desktop")]
impl<'c, 'a> FromCall<'c> for tauri::ipc::Request<'a> {
    fn from_call(call: &'c Call, _key: &str) -> Result<Self, Failure> {
        Err(Failure::refused(
            codes::UNAVAILABLE,
            format!(
                "{}: Tauri's request is not made at a distance (take a `remoteapi::Body`)",
                call.command
            ),
        ))
    }
}

/// How `#[tauri::command]` reads an argument for the remote API: by [`FromCall`] when its type has it (the states,
/// the channels), else from the JSON as the IPC deserializes it. Chosen by the autoref of the method call
/// (`(&Tag::<T>::new()).take(…)`): it sees through the aliases (`CoreState`, `VoiceState`…).
pub mod arg {
    use super::{Call, Failure, FromCall};
    use serde::de::DeserializeOwned;
    use serde_json::Value;
    use std::marker::PhantomData;

    pub struct Tag<T>(PhantomData<fn() -> T>);

    impl<T> Tag<T> {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            Tag(PhantomData)
        }
    }

    pub trait ViaCall<'c, T> {
        fn take(&self, call: &'c Call, key: &str) -> Result<T, Failure>;
    }

    impl<'c, T: FromCall<'c>> ViaCall<'c, T> for Tag<T> {
        fn take(&self, call: &'c Call, key: &str) -> Result<T, Failure> {
            T::from_call(call, key)
        }
    }

    pub trait ViaJson<T> {
        fn take(&self, call: &Call, key: &str) -> Result<T, Failure>;
    }

    /// As Tauri reads an argument: missing, it is `null` (an `Option` is `None`), else an error that says which.
    impl<T: DeserializeOwned> ViaJson<T> for &Tag<T> {
        fn take(&self, call: &Call, key: &str) -> Result<T, Failure> {
            let value = call.args.get(key).cloned();
            let missing = value.is_none();
            T::deserialize(value.unwrap_or(Value::Null)).map_err(|e| {
                Failure::Command(Value::String(if missing {
                    format!("command {} missing required key {key}", call.command)
                } else {
                    format!("invalid args `{key}` for command `{}`: {e}", call.command)
                }))
            })
        }
    }
}

/// How the remote API answers a command's result, as the IPC does: a `Result`'s error serialized, a value (or a
/// `tauri::ipc::Response`'s bytes) by Tauri's `IpcResponse`.
pub mod out {
    use super::{Failure, Outcome};
    use serde::Serialize;
    use std::cell::Cell;
    use tauri::ipc::IpcResponse;

    pub struct Out<T>(Cell<Option<T>>);

    impl<T> Out<T> {
        pub fn new(value: T) -> Self {
            Out(Cell::new(Some(value)))
        }

        fn value(&self) -> T {
            self.0.take().expect("an answer is read once")
        }
    }

    fn body(value: impl IpcResponse) -> Outcome {
        value
            .body()
            .map_err(|e| Failure::Command(serde_json::Value::String(e.to_string())))
    }

    pub trait ViaResult {
        fn reply(&self) -> Outcome;
    }

    impl<T: IpcResponse, E: Serialize> ViaResult for Out<Result<T, E>> {
        fn reply(&self) -> Outcome {
            match self.value() {
                Ok(v) => body(v),
                Err(e) => Err(Failure::Command(
                    serde_json::to_value(e).unwrap_or(serde_json::Value::Null),
                )),
            }
        }
    }

    pub trait ViaValue {
        fn reply(&self) -> Outcome;
    }

    impl<T: IpcResponse> ViaValue for &Out<T> {
        fn reply(&self) -> Outcome {
            body(self.value())
        }
    }
}
