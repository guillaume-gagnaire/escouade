//! The routes of the API, version 1. Every one but `/healthz`, the enrolment by a token and the two steps of the
//! signed challenge asks for a session (`auth::Authed`).

use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::auth::{self, Authed};
use crate::db::{Entry, Kind, NewEntry, Page, Quotas, Role, Snapshot, StoreError, Stream};
use crate::live::Event;
use crate::metrics::Metrics;
use crate::transport::{body, Cbor, Problem};
use crate::util::{id_from, id_from_hex, new_token, now, token_hash, Id};
use crate::Shared;

/// The bodies, as they travel (CBOR). Public for the client to come (TS-2) and the tests.
pub mod wire {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct EnrollRequest {
        pub token: String,
        #[serde(with = "serde_bytes")]
        pub public_key: Vec<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct EnrollResponse {
        #[serde(with = "serde_bytes")]
        pub team: Vec<u8>,
        #[serde(with = "serde_bytes")]
        pub device: Vec<u8>,
        pub role: Role,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ChallengeRequest {
        #[serde(with = "serde_bytes")]
        pub device: Vec<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ChallengeResponse {
        #[serde(with = "serde_bytes")]
        pub challenge: Vec<u8>,
        pub expires_in: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct SessionRequest {
        #[serde(with = "serde_bytes")]
        pub device: Vec<u8>,
        #[serde(with = "serde_bytes")]
        pub challenge: Vec<u8>,
        #[serde(with = "serde_bytes")]
        pub signature: Vec<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TokenResponse {
        pub token: String,
        pub expires_in: u64,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct EnrollmentRequest {
        #[serde(default)]
        pub expires_in: Option<u64>,
        /// The SHA-256 of a token the client made itself (TS-3: the invitation's, derived from the secret after the
        /// `#`, which the server never sees). The answer then carries no token.
        #[serde(default, with = "opt_bytes", skip_serializing_if = "Option::is_none")]
        pub token_hash: Option<Vec<u8>>,
    }

    /// The token's SHA-256 the administrator gave, registered.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Registered {
        pub expires_in: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct RoleRequest {
        pub role: Role,
    }

    /// A token used again after its first use: the SHA-256 the administrator registered, and when.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TokenAlertInfo {
        #[serde(with = "serde_bytes")]
        pub token_hash: Vec<u8>,
        pub used_at: i64,
        pub reused_at: i64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TokenAlerts {
        pub alerts: Vec<TokenAlertInfo>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PairOpen {
        #[serde(with = "serde_bytes")]
        pub message: Vec<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PairOpened {
        pub slot: String,
        pub expires_in: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PairMessage {
        #[serde(with = "serde_bytes")]
        pub message: Vec<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PairTry {
        #[serde(with = "serde_bytes")]
        pub message: Vec<u8>,
        #[serde(with = "serde_bytes")]
        pub confirm: Vec<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PairAnswer {
        #[serde(with = "serde_bytes")]
        pub sealed: Vec<u8>,
    }

    pub mod opt_bytes {
        pub fn serialize<S: serde::Serializer>(
            v: &Option<Vec<u8>>,
            s: S,
        ) -> Result<S::Ok, S::Error> {
            match v {
                Some(b) => s.serialize_bytes(b),
                None => s.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: serde::Deserializer<'de>>(
            d: D,
        ) -> Result<Option<Vec<u8>>, D::Error> {
            let v: Option<serde_bytes::ByteBuf> = serde::Deserialize::deserialize(d)?;
            Ok(v.map(|b| b.into_vec()))
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DeviceInfo {
        #[serde(with = "serde_bytes")]
        pub id: Vec<u8>,
        #[serde(with = "serde_bytes")]
        pub public_key: Vec<u8>,
        pub role: Role,
        pub revoked: bool,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Devices {
        pub devices: Vec<DeviceInfo>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct AppendRequest {
        pub kind: Kind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub epoch: Option<u64>,
        #[serde(with = "serde_bytes")]
        pub payload: Vec<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Appended {
        pub seq: u64,
    }

    /// The detail of a refused commit: the epoch the server waits for, and the entries from the commit that won.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct EpochTaken {
        pub expected_epoch: u64,
        pub preceding: Vec<Entry>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Entries {
        pub entries: Vec<Entry>,
        pub more: bool,
        /// A project's stream: the number its last snapshot holds up to, when it is after `since` (its messages up
        /// to there are forgotten: start from the snapshot).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub snapshot: Option<u64>,
    }

    /// A project's snapshot, as a device sends it.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SnapshotPut {
        pub upto: u64,
        #[serde(with = "serde_bytes")]
        pub payload: Vec<u8>,
    }

    /// A project's snapshot, as the server serves it.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SnapshotInfo {
        pub upto: u64,
        #[serde(with = "serde_bytes")]
        pub device: Vec<u8>,
        pub at: i64,
        #[serde(with = "serde_bytes")]
        pub payload: Vec<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct SnapshotKept {
        /// The messages of the stream forgotten.
        pub pruned: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Usage {
        pub used: u64,
        pub quota: u64,
        pub max_file: u64,
        pub max_entry: u64,
    }
}

use wire::*;

fn device_id(bytes: &[u8]) -> Result<Id, Problem> {
    id_from(bytes).ok_or_else(|| Problem::bad_request("bad_id", "an id is 16 bytes"))
}

fn path_id(text: &str) -> Result<Id, Problem> {
    id_from_hex(text)
        .ok_or_else(|| Problem::bad_request("bad_id", "an id is 32 hexadecimal digits"))
}

fn store_problem(e: StoreError) -> Problem {
    match e {
        StoreError::BadToken => Problem::new(
            StatusCode::UNAUTHORIZED,
            "bad_token",
            "this token is unknown, used or expired",
        ),
        StoreError::TokenUsed => Problem::new(
            StatusCode::CONFLICT,
            "token_used",
            "this token was already used",
        ),
        StoreError::LastAdmin => Problem::new(
            StatusCode::CONFLICT,
            "last_admin",
            "a team keeps one administrator on the server at least",
        ),
        StoreError::KeyTaken => Problem::new(
            StatusCode::CONFLICT,
            "key_taken",
            "this key already has a device",
        ),
        StoreError::Snapshot(why) => Problem::new(StatusCode::CONFLICT, "snapshot_refused", why),
        StoreError::Quota => Problem::new(
            StatusCode::INSUFFICIENT_STORAGE,
            "quota",
            "the team's quota is reached",
        ),
        StoreError::TooMany(what) => Problem::new(
            StatusCode::FORBIDDEN,
            "too_many",
            format!("too many {what}"),
        ),
        StoreError::Exists => Problem::new(StatusCode::CONFLICT, "exists", "it exists already"),
        StoreError::EpochTaken {
            expected,
            preceding,
            ..
        } => Problem::new(
            StatusCode::CONFLICT,
            "epoch_taken",
            "a commit already holds this epoch: process it first",
        )
        .with_detail(&EpochTaken {
            expected_epoch: expected,
            preceding,
        }),
        StoreError::Backend(e) => {
            tracing::error!("base: {e}");
            Problem::internal()
        }
    }
}

impl crate::App {
    fn quotas(&self) -> Quotas {
        Quotas {
            team_bytes: self.limits.team_quota,
            devices: self.limits.max_devices,
            projects: self.limits.max_projects,
        }
    }

    fn page(&self, limit: Option<u32>) -> Page {
        Page {
            entries: limit
                .unwrap_or(self.limits.page_entries)
                .clamp(1, self.limits.page_entries),
            bytes: self.limits.page_bytes,
        }
    }
}

pub async fn healthz(State(app): State<Shared>) -> Response {
    match app.store.ping().await {
        Ok(()) => (StatusCode::OK, "ok\n").into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "base unavailable\n").into_response(),
    }
}

pub async fn enroll(
    State(app): State<Shared>,
    Cbor(req): Cbor<EnrollRequest>,
) -> Result<Cbor<EnrollResponse>, Problem> {
    let public_key: [u8; 32] = req
        .public_key
        .as_slice()
        .try_into()
        .ok()
        .filter(|k| ed25519_dalek::VerifyingKey::from_bytes(k).is_ok())
        .ok_or_else(|| Problem::bad_request("bad_key", "the public key is not an Ed25519 key"))?;
    if req.token.len() > 128 {
        return Err(store_problem(StoreError::BadToken));
    }
    let device = app
        .store
        .enroll(token_hash(&req.token), public_key, now(), app.quotas())
        .await
        .map_err(|e| {
            if matches!(e, StoreError::BadToken) {
                Metrics::inc(&app.metrics.auth_failures);
            }
            store_problem(e)
        })?;
    Ok(Cbor(EnrollResponse {
        team: device.team.to_vec(),
        device: device.id.to_vec(),
        role: device.role,
    }))
}

pub async fn challenge(
    State(app): State<Shared>,
    Cbor(req): Cbor<ChallengeRequest>,
) -> Result<Cbor<ChallengeResponse>, Problem> {
    let id = device_id(&req.device)?;
    let known = app.store.device(id).await.map_err(store_problem)?;
    if !known.is_some_and(|d| d.revoked_at.is_none()) {
        Metrics::inc(&app.metrics.auth_failures);
        return Err(Problem::unauthorized());
    }
    let ttl = app.limits.challenge_ttl;
    let challenge = app.sessions.challenge(id, Instant::now() + ttl)?;
    Ok(Cbor(ChallengeResponse {
        challenge: challenge.to_vec(),
        expires_in: ttl.as_secs(),
    }))
}

pub async fn session(
    State(app): State<Shared>,
    Cbor(req): Cbor<SessionRequest>,
) -> Result<Cbor<TokenResponse>, Problem> {
    let refuse = || {
        Metrics::inc(&app.metrics.auth_failures);
        Problem::unauthorized()
    };
    let id = device_id(&req.device)?;
    let challenge: [u8; 32] = req.challenge.as_slice().try_into().map_err(|_| refuse())?;
    if !app.sessions.take_challenge(&id, &challenge) {
        return Err(refuse());
    }
    let device = app
        .store
        .device(id)
        .await
        .map_err(store_problem)?
        .filter(|d| d.revoked_at.is_none())
        .ok_or_else(refuse)?;
    if !auth::verify(&device.public_key, &id, &challenge, &req.signature) {
        return Err(refuse());
    }
    let ttl = app.limits.session_ttl;
    let token = app.sessions.open(id, Instant::now() + ttl)?;
    Ok(Cbor(TokenResponse {
        token,
        expires_in: ttl.as_secs(),
    }))
}

/// An administrator asks for a one-use token that adds a device to the team: the entry point that TS-3's
/// invitations and its « add a device » build on.
pub async fn create_enrollment(
    State(app): State<Shared>,
    authed: Authed,
    Cbor(req): Cbor<EnrollmentRequest>,
) -> Result<Response, Problem> {
    if !authed.is_admin() {
        return Err(Problem::forbidden("only an administrator adds a device"));
    }
    let max = app.limits.enrollment_ttl_max;
    let ttl = req
        .expires_in
        .map_or(Duration::from_secs(24 * 3600), Duration::from_secs)
        .min(max)
        .max(Duration::from_secs(1));
    let given: Option<[u8; 32]> = match &req.token_hash {
        Some(h) => Some(h.as_slice().try_into().map_err(|_| {
            Problem::bad_request("bad_hash", "a token's hash is 32 bytes (SHA-256)")
        })?),
        None => None,
    };
    let token = given.is_none().then(new_token);
    let hash = given.unwrap_or_else(|| token_hash(token.as_deref().unwrap_or_default()));
    app.store
        .add_token(
            hash,
            crate::db::TokenKind::Device,
            Some(authed.device.team),
            Some(now() + ttl.as_secs() as i64),
            now(),
        )
        .await
        .map_err(|e| match e {
            // The hash given is already a token's: never told whose.
            StoreError::Backend(_) if given.is_some() => Problem::new(
                StatusCode::CONFLICT,
                "exists",
                "this token is known already",
            ),
            e => store_problem(e),
        })?;
    Ok(match token {
        Some(token) => Cbor(TokenResponse {
            token,
            expires_in: ttl.as_secs(),
        })
        .into_response(),
        None => Cbor(Registered {
            expires_in: ttl.as_secs(),
        })
        .into_response(),
    })
}

/// The team's tokens used a second time (a leaked invitation link): for its administrators, who registered them.
pub async fn token_alerts(
    State(app): State<Shared>,
    authed: Authed,
) -> Result<Cbor<TokenAlerts>, Problem> {
    if !authed.is_admin() {
        return Err(Problem::forbidden("only an administrator reads the alerts"));
    }
    let alerts = app
        .store
        .token_alerts(authed.device.team)
        .await
        .map_err(store_problem)?;
    Ok(Cbor(TokenAlerts {
        alerts: alerts
            .into_iter()
            .map(|a| TokenAlertInfo {
                token_hash: a.hash.to_vec(),
                used_at: a.used_at,
                reused_at: a.reused_at,
            })
            .collect(),
    }))
}

/// An administrator sets the role of a device on the server: the server's roles kept in step with the roster
/// (TS-3). The server does not read the roster: the administrator's device does, and asks.
pub async fn set_role(
    State(app): State<Shared>,
    authed: Authed,
    Path(device): Path<String>,
    Cbor(req): Cbor<RoleRequest>,
) -> Result<StatusCode, Problem> {
    let device = path_id(&device)?;
    if !authed.is_admin() {
        return Err(Problem::forbidden("only an administrator changes a role"));
    }
    if app
        .store
        .set_role(authed.device.team, device, req.role)
        .await
        .map_err(store_problem)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Problem::not_found())
    }
}

pub async fn devices(State(app): State<Shared>, authed: Authed) -> Result<Cbor<Devices>, Problem> {
    let devices = app
        .store
        .devices(authed.device.team)
        .await
        .map_err(store_problem)?;
    Ok(Cbor(Devices {
        devices: devices
            .into_iter()
            .map(|d| DeviceInfo {
                id: d.id.to_vec(),
                public_key: d.public_key.to_vec(),
                role: d.role,
                revoked: d.revoked_at.is_some(),
            })
            .collect(),
    }))
}

/// An administrator revokes a device of the team, or a device itself. Its sessions and live connections end.
pub async fn revoke(
    State(app): State<Shared>,
    authed: Authed,
    Path(device): Path<String>,
) -> Result<StatusCode, Problem> {
    let device = path_id(&device)?;
    if device != authed.device.id && !authed.is_admin() {
        return Err(Problem::forbidden(
            "only an administrator revokes another device",
        ));
    }
    let team = authed.device.team;
    if !app
        .store
        .revoke(team, device, now())
        .await
        .map_err(store_problem)?
    {
        return Err(Problem::not_found());
    }
    app.sessions.close_device(&device);
    app.hub.publish(&team, Event::Revoked(device));
    Ok(StatusCode::NO_CONTENT)
}

// === The pairing relay (`pair.rs`) ===

fn pair_problem(e: crate::pair::PairError) -> Problem {
    use crate::pair::PairError;
    match e {
        PairError::Gone => Problem::new(
            StatusCode::NOT_FOUND,
            "pair_gone",
            "no such code: its time is over, or it served",
        ),
        PairError::Tried => Problem::new(
            StatusCode::CONFLICT,
            "pair_tried",
            "this code had its one try: ask for a new one",
        ),
        PairError::NotYours => Problem::forbidden("this code is another device's"),
        PairError::Full => Problem::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "busy",
            "too many codes open: try again",
        ),
    }
}

fn bounded(bytes: &[u8], max: usize) -> Result<(), Problem> {
    if bytes.is_empty() || bytes.len() > max {
        return Err(Problem::bad_request(
            "bad_size",
            "a message of the wrong size",
        ));
    }
    Ok(())
}

pub async fn pair_open(
    State(app): State<Shared>,
    authed: Authed,
    Cbor(req): Cbor<PairOpen>,
) -> Result<Cbor<PairOpened>, Problem> {
    bounded(&req.message, crate::pair::MAX_MESSAGE)?;
    let slot = app
        .pairs
        .open(authed.device.team, authed.device.id, req.message)
        .map_err(pair_problem)?;
    Ok(Cbor(PairOpened {
        slot,
        expires_in: crate::pair::TTL.as_secs(),
    }))
}

pub async fn pair_message(
    State(app): State<Shared>,
    Path(slot): Path<String>,
) -> Result<Cbor<PairMessage>, Problem> {
    let message = app.pairs.message(&slot).map_err(pair_problem)?;
    Ok(Cbor(PairMessage { message }))
}

pub async fn pair_try(
    State(app): State<Shared>,
    Path(slot): Path<String>,
    Cbor(req): Cbor<PairTry>,
) -> Result<StatusCode, Problem> {
    bounded(&req.message, crate::pair::MAX_MESSAGE)?;
    bounded(&req.confirm, crate::pair::MAX_MESSAGE)?;
    app.pairs
        .try_once(&slot, req.message, req.confirm)
        .map_err(pair_problem)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn pair_tried(
    State(app): State<Shared>,
    authed: Authed,
    Path(slot): Path<String>,
) -> Result<Response, Problem> {
    match app
        .pairs
        .tried(&slot, &authed.device.id)
        .map_err(pair_problem)?
    {
        Some((message, confirm)) => Ok(Cbor(PairTry { message, confirm }).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

pub async fn pair_answer(
    State(app): State<Shared>,
    authed: Authed,
    Path(slot): Path<String>,
    Cbor(req): Cbor<PairAnswer>,
) -> Result<StatusCode, Problem> {
    bounded(&req.sealed, crate::pair::MAX_ANSWER)?;
    app.pairs
        .answer(&slot, &authed.device.id, req.sealed)
        .map_err(pair_problem)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn pair_take(
    State(app): State<Shared>,
    Path(slot): Path<String>,
) -> Result<Response, Problem> {
    match app.pairs.take_answer(&slot).map_err(pair_problem)? {
        Some(sealed) => Ok(Cbor(PairAnswer { sealed }).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

pub async fn pair_close(
    State(app): State<Shared>,
    authed: Authed,
    Path(slot): Path<String>,
) -> Result<StatusCode, Problem> {
    app.pairs
        .close(&slot, &authed.device.id)
        .map_err(pair_problem)?;
    Ok(StatusCode::NO_CONTENT)
}

// === The page of an invitation link ===

/// `/join/<id>`: an invitation link opened in a browser. The secret after the `#` never reaches the server; the
/// page's script reads it where it is (`location.hash`) and hands the whole link to Escouade
/// (`escouade://team/join/<server>/<id>#…`). No resource from elsewhere, a script by its nonce alone.
pub async fn join_page(Path(id): Path<String>) -> Result<Response, Problem> {
    path_id(&id)?;
    let nonce = hex::encode(crate::util::random::<16>());
    let page = JOIN_PAGE.replace("{nonce}", &nonce);
    let policy = format!(
        "default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-{nonce}'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
    );
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_str(&policy).expect("a policy is a header value"),
            ),
            (
                header::REFERRER_POLICY,
                HeaderValue::from_static("no-referrer"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        page,
    )
        .into_response())
}

const JOIN_PAGE: &str = include_str!("join.html");

/// The stream of the team's list of projects (`escouade_team::catalog::CATALOG_STREAM`): its content is encrypted,
/// only its writers are known here.
pub const CATALOG_STREAM: Id = *b"escouade/project";

async fn append(
    app: &Shared,
    authed: &Authed,
    stream: Stream,
    req: AppendRequest,
) -> Result<Cbor<Appended>, Problem> {
    if req.payload.len() > app.limits.max_entry {
        return Err(Problem::too_large(app.limits.max_entry));
    }
    if req.payload.is_empty() {
        return Err(Problem::bad_request("empty", "an entry carries bytes"));
    }
    match (stream, req.kind, req.epoch) {
        (_, Kind::Commit, Some(_)) | (_, Kind::Message, None) => {}
        (_, Kind::Commit, None) => {
            return Err(Problem::bad_request("no_epoch", "a commit says its epoch"))
        }
        (_, Kind::Message, Some(_)) => {
            return Err(Problem::bad_request(
                "epoch_on_message",
                "only a commit has an epoch",
            ))
        }
    }
    let team = authed.device.team;
    let size = req.payload.len() as u64;
    let entry = NewEntry {
        kind: req.kind,
        epoch: req.epoch,
        device: authed.device.id,
        at: now(),
        payload: req.payload,
    };
    let entry = app
        .store
        .append(team, stream, entry, app.quotas(), app.page(None))
        .await
        .map_err(|e| {
            if matches!(e, StoreError::EpochTaken { .. }) {
                Metrics::inc(&app.metrics.epoch_conflicts);
            }
            store_problem(e)
        })?;
    let seq = entry.seq;
    match stream {
        Stream::Team => {
            Metrics::inc(&app.metrics.team_entries);
            app.hub.publish(&team, Event::Team(entry));
        }
        Stream::Project(p) => {
            Metrics::inc(&app.metrics.project_entries);
            app.hub.publish(&team, Event::Project(p, entry));
        }
    }
    app.metrics
        .entry_bytes
        .fetch_add(size, std::sync::atomic::Ordering::Relaxed);
    Ok(Cbor(Appended { seq }))
}

pub async fn append_team(
    State(app): State<Shared>,
    authed: Authed,
    Cbor(req): Cbor<AppendRequest>,
) -> Result<Cbor<Appended>, Problem> {
    append(&app, &authed, Stream::Team, req).await
}

pub async fn append_project(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Cbor(req): Cbor<AppendRequest>,
) -> Result<Cbor<Appended>, Problem> {
    let project = path_id(&project)?;
    // The team's list of projects (TS-8) is its administrators': the clients refuse a member's entry at the reading,
    // and the server, which knows the roles, does not even keep one (defense in depth).
    if project == CATALOG_STREAM && !authed.is_admin() {
        return Err(Problem::forbidden(
            "only an administrator writes the team's list of projects",
        ));
    }
    append(&app, &authed, Stream::Project(project), req).await
}

#[derive(Debug, Deserialize)]
pub struct Since {
    #[serde(default)]
    pub since: u64,
    #[serde(default)]
    pub limit: Option<u32>,
}

pub async fn read_team(
    State(app): State<Shared>,
    authed: Authed,
    Query(q): Query<Since>,
) -> Result<Cbor<Entries>, Problem> {
    let (entries, more) = app
        .store
        .read(authed.device.team, Stream::Team, q.since, app.page(q.limit))
        .await
        .map_err(store_problem)?;
    Ok(Cbor(Entries {
        entries,
        more,
        snapshot: None,
    }))
}

pub async fn read_project(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Query(q): Query<Since>,
) -> Result<Cbor<Entries>, Problem> {
    let project = path_id(&project)?;
    let (entries, more) = app
        .store
        .read(
            authed.device.team,
            Stream::Project(project),
            q.since,
            app.page(q.limit),
        )
        .await
        .map_err(store_problem)?;
    let snapshot = app
        .store
        .snapshot_upto(authed.device.team, project)
        .await
        .map_err(store_problem)?
        .filter(|upto| *upto > q.since);
    Ok(Cbor(Entries {
        entries,
        more,
        snapshot,
    }))
}

/// Keeps a project's snapshot (TS-5): its messages up to `upto` are forgotten, its commits stay. A snapshot not
/// after the one kept, or beyond the stream, is refused (409 `snapshot_refused`).
pub async fn put_snapshot(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Cbor(req): Cbor<SnapshotPut>,
) -> Result<Cbor<SnapshotKept>, Problem> {
    let project = path_id(&project)?;
    if req.payload.is_empty() {
        return Err(Problem::bad_request("empty", "a snapshot carries bytes"));
    }
    if req.payload.len() > app.limits.max_file {
        return Err(Problem::too_large(app.limits.max_file));
    }
    let pruned = app
        .store
        .put_snapshot(
            authed.device.team,
            project,
            Snapshot {
                upto: req.upto,
                device: authed.device.id,
                at: now(),
                payload: req.payload,
            },
            app.quotas(),
        )
        .await
        .map_err(store_problem)?;
    Ok(Cbor(SnapshotKept { pruned }))
}

pub async fn get_snapshot(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
) -> Result<Cbor<SnapshotInfo>, Problem> {
    let project = path_id(&project)?;
    let s = app
        .store
        .snapshot(authed.device.team, project)
        .await
        .map_err(store_problem)?
        .ok_or_else(Problem::not_found)?;
    Ok(Cbor(SnapshotInfo {
        upto: s.upto,
        device: s.device.to_vec(),
        at: s.at,
        payload: s.payload,
    }))
}

pub async fn put_file(
    State(app): State<Shared>,
    authed: Authed,
    Path(file): Path<String>,
    req: Request,
) -> Result<StatusCode, Problem> {
    let file = path_id(&file)?;
    let max = app.limits.max_file;
    let bytes: Bytes = body::read(req, max).await?;
    if bytes.is_empty() {
        return Err(Problem::bad_request("empty", "a file carries bytes"));
    }
    let team = authed.device.team;
    app.store
        .reserve_file(
            team,
            file,
            bytes.len() as u64,
            authed.device.id,
            now(),
            app.quotas(),
        )
        .await
        .map_err(store_problem)?;
    let size = bytes.len() as u64;
    if let Err(e) = app.blobs.put(&team, &file, bytes).await {
        tracing::error!("{e}");
        let _ = app.store.delete_file(team, file).await;
        return Err(Problem::internal());
    }
    Metrics::inc(&app.metrics.files_written);
    app.metrics
        .file_bytes
        .fetch_add(size, std::sync::atomic::Ordering::Relaxed);
    Ok(StatusCode::CREATED)
}

pub async fn get_file(
    State(app): State<Shared>,
    authed: Authed,
    Path(file): Path<String>,
) -> Result<Response, Problem> {
    let file = path_id(&file)?;
    let team = authed.device.team;
    if app
        .store
        .file_size(team, file)
        .await
        .map_err(store_problem)?
        .is_none()
    {
        return Err(Problem::not_found());
    }
    let bytes = app
        .blobs
        .get(&team, &file)
        .await
        .map_err(|e| {
            tracing::error!("{e}");
            Problem::internal()
        })?
        .ok_or_else(Problem::not_found)?;
    Ok((
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        )],
        Body::from(bytes),
    )
        .into_response())
}

pub async fn delete_file(
    State(app): State<Shared>,
    authed: Authed,
    Path(file): Path<String>,
) -> Result<StatusCode, Problem> {
    let file = path_id(&file)?;
    let team = authed.device.team;
    if app
        .store
        .delete_file(team, file)
        .await
        .map_err(store_problem)?
        .is_none()
    {
        return Err(Problem::not_found());
    }
    if let Err(e) = app.blobs.delete(&team, &file).await {
        // The place is given back; the bytes left behind are encrypted, and a sweep can take them later.
        tracing::warn!("{e}");
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn usage(State(app): State<Shared>, authed: Authed) -> Result<Cbor<Usage>, Problem> {
    let used = app
        .store
        .used_bytes(authed.device.team)
        .await
        .map_err(store_problem)?;
    Ok(Cbor(Usage {
        used,
        quota: app.limits.team_quota,
        max_file: app.limits.max_file as u64,
        max_entry: app.limits.max_entry as u64,
    }))
}
