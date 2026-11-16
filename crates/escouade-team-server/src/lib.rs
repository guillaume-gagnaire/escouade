//! Escouade's team server (TS-1, `docs/TEAM-SERVER.md` §2.1, §2.3, §2.6, §3.8, §14 and §16).
//!
//! **Blind**: it keeps encrypted bytes and the metadata of §3.8, runs nothing (no agent, command, plugin or harness)
//! and holds no content key. It knows devices (an id, an Ed25519 public key, a team, a role, revoked or not), signs
//! them in by a challenge, numbers and keeps the team's stream (the MLS messages: the first commit of an epoch wins)
//! and each project's stream (TS-5's encrypted operations), keeps encrypted files on the disk or in S3, and
//! broadcasts the new entries by WebSocket. `README.md` says what it sees, how to install it, and what is left.

pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod files;
pub mod live;
pub mod metrics;
pub mod pair;
pub mod transport;
pub mod util;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, MatchedPath, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Router};

use crate::config::{Config, Limits};
use crate::db::{Store, TokenKind};
use crate::files::Blobs;
use crate::metrics::Metrics;
use crate::transport::rate::{self, Limiter};
use crate::transport::{body, handshake, listen};
use crate::util::Id;

/// The protocol of this server. It accepts this one and the one before (`transport::handshake`).
pub const PROTOCOL: u32 = 1;

pub struct App {
    pub store: Box<dyn Store>,
    pub blobs: Box<dyn Blobs>,
    pub limits: Limits,
    pub sessions: auth::Sessions,
    pub hub: live::Hub,
    pub pairs: pair::Pairs,
    pub metrics: Metrics,
    pub address_rate: Limiter<IpAddr>,
    pub device_rate: Limiter<Id>,
    pub trust_proxy: bool,
}

pub type Shared = Arc<App>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Store(#[from] db::StoreError),
    #[error(transparent)]
    Blobs(#[from] files::BlobError),
}

pub struct Server {
    app: Shared,
}

impl Server {
    /// Opens the base (made if needed) and the file store.
    pub async fn open(config: Config) -> Result<Self, Error> {
        let store = db::open(&config.database).await?;
        let blobs = files::open(&config.files)?;
        let app = Arc::new(App {
            store,
            blobs,
            limits: config.limits,
            sessions: auth::Sessions::default(),
            hub: live::Hub::default(),
            pairs: pair::Pairs::default(),
            metrics: Metrics::default(),
            address_rate: Limiter::new(config.rates.per_address),
            device_rate: Limiter::new(config.rates.per_device),
            trust_proxy: config.trust_proxy,
        });
        Ok(Server { app })
    }

    pub fn app(&self) -> &Shared {
        &self.app
    }

    /// A one-use token that creates a team and its first device (`init`). Only its SHA-256 is kept.
    pub async fn new_team_token(&self) -> Result<String, Error> {
        let token = util::new_token();
        self.app
            .store
            .add_token(
                util::token_hash(&token),
                TokenKind::Team,
                None,
                None,
                util::now(),
            )
            .await?;
        Ok(token)
    }

    /// A one-use token that adds a device to `team` as an administrator of the server
    /// (`escouade-team-server recovery-token`): for the holder of the team's recovery key, the day every administrator
    /// lost their devices (TS-3, `docs/TEAM-SERVER.md` §3.5). It gives no key: the words of the recovery key do. It
    /// lasts `enrollment_ttl_max`.
    pub async fn new_recovery_token(&self, team: Id) -> Result<String, Error> {
        let token = util::new_token();
        let now = util::now();
        self.app
            .store
            .add_token(
                util::token_hash(&token),
                TokenKind::Recovery,
                Some(team),
                Some(now + self.app.limits.enrollment_ttl_max.as_secs() as i64),
                now,
            )
            .await?;
        Ok(token)
    }

    /// The exposed routes.
    pub fn router(&self) -> Router {
        let limits = &self.app.limits;
        let entry_max = body::Max(limits.max_entry + (4 << 10));
        let v1 = Router::new()
            .route("/v1/enroll", post(api::enroll))
            .route("/v1/auth/challenge", post(api::challenge))
            .route("/v1/auth/session", post(api::session))
            .route("/v1/devices", get(api::devices))
            .route("/v1/devices/enrollments", post(api::create_enrollment))
            .route("/v1/devices/enrollments/alerts", get(api::token_alerts))
            .route("/v1/devices/{device}/revoke", post(api::revoke))
            .route("/v1/devices/{device}/role", post(api::set_role))
            .route("/v1/pair", post(api::pair_open))
            .route(
                "/v1/pair/{slot}",
                get(api::pair_message).delete(api::pair_close),
            )
            .route(
                "/v1/pair/{slot}/try",
                get(api::pair_tried).post(api::pair_try),
            )
            .route(
                "/v1/pair/{slot}/answer",
                get(api::pair_take).post(api::pair_answer),
            )
            .route(
                "/v1/team/stream",
                get(api::read_team)
                    .post(api::append_team)
                    .layer(Extension(entry_max)),
            )
            .route(
                "/v1/projects/{project}/stream",
                get(api::read_project)
                    .post(api::append_project)
                    .layer(Extension(entry_max)),
            )
            .route(
                "/v1/projects/{project}/access",
                get(api::get_access).put(api::put_access),
            )
            .route("/v1/projects/{project}/claims", get(api::list_claims))
            .route(
                "/v1/projects/{project}/claims/{ticket}",
                post(api::take_claim),
            )
            .route(
                "/v1/projects/{project}/claims/{ticket}/renew",
                post(api::renew_claim),
            )
            .route(
                "/v1/projects/{project}/claims/{ticket}/release",
                post(api::release_claim),
            )
            .route("/v1/projects/{project}/keys", post(api::next_key))
            .route(
                "/v1/projects/{project}/snapshot",
                get(api::get_snapshot)
                    .put(api::put_snapshot)
                    .layer(Extension(body::Max(limits.max_file + (4 << 10)))),
            )
            .route("/v1/files/usage", get(api::usage))
            .route(
                "/v1/files/{file}",
                get(api::get_file)
                    .put(api::put_file)
                    .delete(api::delete_file),
            )
            .route("/v1/live", get(live::upgrade))
            .layer(middleware::from_fn_with_state(PROTOCOL, handshake::layer));
        Router::new()
            .route("/healthz", get(api::healthz))
            .route("/join/{id}", get(api::join_page))
            .merge(v1)
            .fallback(|| async { transport::Problem::not_found() })
            .layer(middleware::from_fn_with_state(
                self.app.clone(),
                limit_address,
            ))
            .layer(middleware::from_fn_with_state(self.app.clone(), observe))
            .layer(axum::extract::DefaultBodyLimit::disable())
            .with_state(self.app.clone())
    }

    /// The metrics, for their own address.
    pub fn metrics_router(&self) -> Router {
        Router::new()
            .route("/metrics", get(render_metrics))
            .with_state(self.app.clone())
    }

    /// Forgets the expired challenges and sessions, the full buckets, the teams nobody listens to, the pairing codes and
    /// the used tokens whose time is over, every 30 s.
    pub fn spawn_sweeper(&self) -> tokio::task::JoinHandle<()> {
        let app = self.app.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(30));
            loop {
                tick.tick().await;
                app.sessions.sweep();
                app.address_rate.sweep();
                app.device_rate.sweep();
                app.hub.sweep();
                app.pairs.sweep();
                if let Err(e) = app.store.sweep_tokens(util::now()).await {
                    tracing::warn!("tokens: {e}");
                }
            }
        })
    }

    /// Serves until `shutdown`.
    pub async fn serve(
        &self,
        listener: tokio::net::TcpListener,
        tls: Option<Arc<rustls::ServerConfig>>,
        shutdown: impl std::future::Future<Output = ()> + Send,
    ) -> std::io::Result<()> {
        listen::serve(listener, self.router(), tls, shutdown).await
    }
}

async fn render_metrics(State(app): State<Shared>) -> Response {
    let counts = app.store.counts().await.ok();
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        app.metrics.render(counts),
    )
        .into_response()
}

/// The rate by address, before anything else.
async fn limit_address(State(app): State<Shared>, req: Request, next: Next) -> Response {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0);
    let forwarded = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok());
    if let Some(ip) = listen::client_address(peer, forwarded, app.trust_proxy) {
        if let Err(wait) = app.address_rate.take(&ip) {
            Metrics::inc(&app.metrics.rate_limited);
            return rate::refused(wait).into_response();
        }
    }
    next.run(req).await
}

/// Counts and logs each request by its route's template: never a path with an id, an address or a token.
async fn observe(State(app): State<Shared>, req: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = req.method().as_str().to_string();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", |m| m.as_str())
        .to_string();
    let response = next.run(req).await;
    let status = response.status();
    app.metrics.request(&method, &route, status.as_u16());
    let ms = started.elapsed().as_millis() as u64;
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        tracing::warn!(method, route, status = status.as_u16(), ms, "request");
    } else {
        tracing::info!(method, route, status = status.as_u16(), ms, "request");
    }
    response
}
