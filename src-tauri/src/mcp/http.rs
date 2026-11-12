//! The MCP server on HTTP: hyper on 127.0.0.1, every request through the guards of `Gate` before
//! it reaches rmcp's Streamable HTTP service (stateless, JSON answers). A browser's page may send
//! requests to the loopback (DNS rebinding, a form): its `Host` and its `Origin` give it away; any
//! other program needs a token.

use super::activity::{self, clip, Outcome};
use super::tools::{ToolCtx, Tools};
use super::{Caller, PATH};
use crate::core::Core;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::{Body, Bytes, Frame, Incoming, SizeHint};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Weak};
use std::task::{Context, Poll};
use std::time::Duration;
use tauri::Runtime;
use tokio::sync::{watch, Semaphore};

/// How long a connection open when the server stops may finish the request it is answering.
const DRAIN: Duration = Duration::from_secs(5);

/// The most a request may weigh, in bytes (rmcp refuses 4 MiB by default). A ticket takes up to
/// 18 MB of images, which a tool is given in base64 (a third more): 24 MB, and the other arguments
/// are text.
pub(super) const MAX_BODY: usize = 32 * 1024 * 1024;

/// The most a request may weigh when it carries no image or file: a message, a ticket's text, a
/// page of the spec are far below (a message is 32,000 characters). Only the tools that take
/// attachments (`BULKY_TOOLS`) may weigh up to `MAX_BODY`.
pub(super) const MAX_TEXT_BODY: usize = 2 * 1024 * 1024;

/// The tools that take images or files by their content in base64.
const BULKY_TOOLS: [&str; 2] = ["create_ticket", "update_ticket"];

/// What the clients may take of the server: a local program, the token unknown to it, could
/// otherwise hold it with connections that never end their request.
#[derive(Debug, Clone)]
pub(super) struct Limits {
    /// How long a connection has to send a request's headers (also how long it may stay idle
    /// between two requests): then it is closed.
    pub header_read: Duration,
    /// Connections open at once; the next ones wait (the system queues them) until one closes.
    pub connections: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            // A client opens a connection to send its request at once.
            header_read: Duration::from_secs(10),
            // Each Claude Code (a terminal, every agent) keeps one or two open.
            connections: 64,
        }
    }
}

/// The guards and the MCP service behind them, for the server listening on `port`.
pub(super) struct Gate<R: Runtime> {
    port: u16,
    core: Weak<Core<R>>,
    mcp: StreamableHttpService<Tools<R>, NeverSessionManager>,
    /// The tools the server has: a `tools/call` of any other name never reaches rmcp.
    tools: ToolRouter<Tools<R>>,
}

/// Why a request was refused before rmcp.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Refusal {
    /// It has no `Host` (and no absolute URI).
    NoHost,
    /// Its `Host` (shown) is not the server's.
    Host(String),
    /// It has an `Origin` (shown): a browser's page sent it.
    Origin(String),
    NoToken,
    UnknownToken,
    /// Not the MCP endpoint (its path, shown).
    Path(String),
    /// A `tools/call` of a tool the server does not have (its name is the client’s: never repeated).
    UnknownTool,
    /// A request without image or file weighing more than `MAX_TEXT_BODY` (or one that weighs
    /// more than `MAX_BODY`).
    TooLarge,
    /// The body could not be read.
    Unreadable,
}

impl Refusal {
    fn status(&self) -> StatusCode {
        match self {
            Refusal::NoHost | Refusal::Host(_) | Refusal::Origin(_) => StatusCode::FORBIDDEN,
            Refusal::NoToken | Refusal::UnknownToken => StatusCode::UNAUTHORIZED,
            Refusal::Path(_) => StatusCode::NOT_FOUND,
            Refusal::UnknownTool | Refusal::Unreadable => StatusCode::BAD_REQUEST,
            Refusal::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }

    /// What the activity log and the answer say.
    fn message(&self) -> String {
        match self {
            Refusal::NoHost => tr!(
                "Requête refusée : pas d’en-tête Host",
                "Request refused: no Host header"
            ),
            Refusal::Host(host) => tr!(
                "Requête refusée : en-tête Host inattendu ({host})",
                "Request refused: unexpected Host header ({host})"
            ),
            Refusal::Origin(origin) => tr!(
                "Requête refusée : en-tête Origin d’une page web ({origin})",
                "Request refused: Origin header of a web page ({origin})"
            ),
            Refusal::NoToken => tr!(
                "Requête refusée : pas de jeton",
                "Request refused: no token"
            ),
            Refusal::UnknownToken => tr!(
                "Requête refusée : jeton inconnu",
                "Request refused: unknown token"
            ),
            Refusal::Path(path) => tr!(
                "Requête refusée : chemin inconnu ({path})",
                "Request refused: unknown path ({path})"
            ),
            Refusal::UnknownTool => tr!(
                "Requête refusée : outil inconnu",
                "Request refused: unknown tool"
            ),
            Refusal::TooLarge => tr!(
                "Requête refusée : trop volumineuse",
                "Request refused: too large"
            ),
            Refusal::Unreadable => tr!(
                "Requête refusée : corps illisible",
                "Request refused: unreadable body"
            ),
        }
    }
}

impl<R: Runtime> Gate<R> {
    pub(super) fn new(port: u16, core: Weak<Core<R>>) -> Self {
        let router = Tools::router();
        let tools = router.clone();
        // rmcp's own checks of the Host and the Origin stay on behind ours.
        let config = StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .with_sse_keep_alive(None)
            .with_allowed_hosts([format!("127.0.0.1:{port}"), format!("localhost:{port}")])
            .with_max_request_body_bytes(MAX_BODY)
            .enforce_origin_validation();
        let mcp = StreamableHttpService::new(
            move || Ok(Tools::new(router.clone())),
            Arc::new(NeverSessionManager::default()),
            config,
        );
        Self {
            port,
            core,
            mcp,
            tools,
        }
    }

    /// The answer to `req`: a refusal (logged), or rmcp's, the caller in the request's extensions
    /// for the tools (`ToolCtx`).
    async fn answer(&self, req: Request<Incoming>) -> Response<Reply> {
        let Some(core) = self.core.upgrade() else {
            return text(StatusCode::SERVICE_UNAVAILABLE, String::new());
        };
        let asked = format!("{} {}", req.method(), req.uri().path());
        let (caller, mut req) = match self.admit(&core, req).await {
            Ok(admitted) => admitted,
            Err((caller, refusal)) => {
                let message = refusal.message();
                activity::record(
                    &core,
                    caller.as_ref(),
                    "",
                    &asked,
                    Outcome::Refused(message.clone()),
                );
                let mut answer = text(refusal.status(), message);
                if refusal.status() == StatusCode::UNAUTHORIZED {
                    answer.headers_mut().insert(
                        header::WWW_AUTHENTICATE,
                        header::HeaderValue::from_static("Bearer"),
                    );
                }
                return answer;
            }
        };
        req.extensions_mut().insert(ToolCtx::new(core, caller));
        self.mcp
            .handle(req)
            .await
            .map(|body| Reply::Mcp(Box::pin(body)))
    }

    /// Who `req` comes from, once its `Host`, its `Origin`, its token, its path and its body
    /// pass, with the request to give rmcp (its body read, `MAX_BODY` at most); else why it does
    /// not, with its caller when its token was read. The body is read for a known caller only: a
    /// `tools/call` must name a tool the server has (rmcp keeps the name of each it is asked,
    /// refused or not), and a request without image or file stays under `MAX_TEXT_BODY`.
    async fn admit(
        &self,
        core: &Core<R>,
        req: Request<Incoming>,
    ) -> Result<(Caller, Request<Full<Bytes>>), (Option<Caller>, Refusal)> {
        let caller = self.admit_head(core, &req)?;
        let too_large = |caller: &Caller| (Some(caller.clone()), Refusal::TooLarge);
        let declared = req
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<usize>().ok());
        if declared.is_some_and(|n| n > MAX_BODY) {
            return Err(too_large(&caller));
        }
        let (parts, body) = req.into_parts();
        let bytes = match Limited::new(body, MAX_BODY).collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e)
                if e.downcast_ref::<http_body_util::LengthLimitError>()
                    .is_some() =>
            {
                return Err(too_large(&caller))
            }
            Err(_) => return Err((Some(caller), Refusal::Unreadable)),
        };
        if let Err(refusal) = check_body(&self.tools, &bytes) {
            return Err((Some(caller), refusal));
        }
        Ok((caller, Request::from_parts(parts, Full::new(bytes))))
    }

    /// Who `req` comes from, once its `Host`, its `Origin`, its token and its path pass; else
    /// why it does not, with its caller when its token was read.
    fn admit_head(
        &self,
        core: &Core<R>,
        req: &Request<Incoming>,
    ) -> Result<Caller, (Option<Caller>, Refusal)> {
        let headers = req.headers();
        let host = headers.get(header::HOST).map(shown);
        // An absolute URI (`POST http://…/mcp`) names the host in place of the header.
        let authority = req.uri().authority().map(|a| a.as_str().to_string());
        if let Some(a) = authority.as_deref().filter(|a| !self.is_mine(a)) {
            return Err((None, Refusal::Host(clip(a))));
        }
        match host {
            None => return Err((None, Refusal::NoHost)),
            Some(h) if !self.is_mine(&h) => return Err((None, Refusal::Host(h))),
            Some(_) => {}
        }
        if let Some(origin) = headers.get(header::ORIGIN) {
            return Err((None, Refusal::Origin(shown(origin))));
        }
        let Some(token) = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(bearer)
        else {
            return Err((None, Refusal::NoToken));
        };
        let Some(caller) = core.mcp.caller(token) else {
            return Err((None, Refusal::UnknownToken));
        };
        if req.uri().path() != PATH {
            return Err((Some(caller), Refusal::Path(clip(req.uri().path()))));
        }
        Ok(caller)
    }

    /// `host` names this server: 127.0.0.1 or localhost, on its port.
    fn is_mine(&self, host: &str) -> bool {
        let port = self.port;
        [format!("127.0.0.1:{port}"), format!("localhost:{port}")]
            .iter()
            .any(|h| h.eq_ignore_ascii_case(host))
    }
}

/// Whether a request's body may reach rmcp: a `tools/call` (alone or in a batch) names a tool of
/// `tools`, and a body that calls none of `BULKY_TOOLS` weighs `MAX_TEXT_BODY` at most. A body
/// that is not JSON is left to rmcp to answer, under the same cap.
fn check_body<S: Send + Sync + 'static>(tools: &ToolRouter<S>, body: &[u8]) -> Result<(), Refusal> {
    let value: serde_json::Value = serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
    let messages = match &value {
        serde_json::Value::Array(list) => list.as_slice(),
        other => std::slice::from_ref(other),
    };
    let mut bulky = false;
    for message in messages {
        if message["method"] != "tools/call" {
            continue;
        }
        // A name that is not a string is no tool's either.
        let name = message["params"]["name"].as_str().unwrap_or_default();
        if !tools.has_route(name) {
            return Err(Refusal::UnknownTool);
        }
        bulky |= BULKY_TOOLS.contains(&name);
    }
    if !bulky && body.len() > MAX_TEXT_BODY {
        return Err(Refusal::TooLarge);
    }
    Ok(())
}

/// The token of an `Authorization: Bearer <token>` header (the scheme in any case).
fn bearer(value: &str) -> Option<&str> {
    let (scheme, token) = value.trim().split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

/// A header's value as the log may show it.
fn shown(value: &header::HeaderValue) -> String {
    clip(&String::from_utf8_lossy(value.as_bytes()))
}

fn text(status: StatusCode, message: String) -> Response<Reply> {
    let mut answer = Response::new(Reply::Text(Some(Bytes::from(message))));
    *answer.status_mut() = status;
    answer.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    answer
}

/// What the server answers: rmcp's body, or a refusal's text.
pub(super) enum Reply {
    Mcp(Pin<Box<dyn Body<Data = Bytes, Error = Infallible> + Send>>),
    Text(Option<Bytes>),
}

impl Body for Reply {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        match self.get_mut() {
            Reply::Mcp(body) => body.as_mut().poll_frame(cx),
            Reply::Text(text) => Poll::Ready(text.take().map(|t| Ok(Frame::data(t)))),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Reply::Mcp(body) => body.is_end_stream(),
            Reply::Text(text) => text.is_none(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Reply::Mcp(body) => body.size_hint(),
            Reply::Text(text) => SizeHint::with_exact(text.as_ref().map_or(0, |t| t.len() as u64)),
        }
    }
}

/// Answers on `listener` until `stop` says so (or its sender goes), then closes it: from then on
/// a connection is refused. The connections open finish the request they are answering
/// (`DRAIN` at most) and close. No more than `limits.connections` at once, each closed when it
/// does not send its headers in time.
pub(super) async fn serve<R: Runtime>(
    listener: std::net::TcpListener,
    gate: Gate<R>,
    mut stop: watch::Receiver<bool>,
    limits: Limits,
) {
    let listener = match listener
        .set_nonblocking(true)
        .and_then(|()| tokio::net::TcpListener::from_std(listener))
    {
        Ok(l) => l,
        Err(e) => {
            log::error!("mcp: cannot listen: {e}");
            return;
        }
    };
    let gate = Arc::new(gate);
    let slots = Arc::new(Semaphore::new(limits.connections.max(1)));
    loop {
        // A free slot before the next connection is taken: beyond, they wait in the system's
        // queue.
        let slot = tokio::select! {
            _ = stop.changed() => break,
            slot = slots.clone().acquire_owned() => match slot {
                Ok(slot) => slot,
                Err(_) => break,
            },
        };
        let accepted = tokio::select! {
            _ = stop.changed() => break,
            accepted = listener.accept() => accepted,
        };
        let stream = match accepted {
            Ok((stream, _)) => stream,
            Err(e) => {
                // Out of sockets for a moment, a connection reset before it was taken.
                log::warn!("mcp: connection not accepted: {e}");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let gate = gate.clone();
        let mut stop = stop.clone();
        let header_read = limits.header_read;
        tauri::async_runtime::spawn(async move {
            // Its slot is free again once the connection closes.
            let _slot = slot;
            let service = service_fn(move |req| {
                let gate = gate.clone();
                async move { Ok::<_, Infallible>(gate.answer(req).await) }
            });
            // Without a timer, hyper's timeout on the headers would not apply.
            let conn = http1::Builder::new()
                .timer(TokioTimer::new())
                .header_read_timeout(header_read)
                .serve_connection(TokioIo::new(stream), service);
            let mut conn = std::pin::pin!(conn);
            tokio::select! {
                _ = conn.as_mut() => {}
                _ = stop.changed() => {
                    conn.as_mut().graceful_shutdown();
                    let _ = tokio::time::timeout(DRAIN, conn).await;
                }
            }
        });
    }
    gate.mcp.config.cancellation_token.cancel();
}
