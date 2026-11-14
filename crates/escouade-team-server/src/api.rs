//! The routes of the API, version 1. Every one but `/healthz`, the enrolment by a token and the two steps of the
//! signed challenge asks for a session (`auth::Authed`).

use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::auth::{self, Authed};
use crate::db::{Entry, Kind, NewEntry, Page, Quotas, Role, StoreError, Stream};
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
        StoreError::KeyTaken => Problem::new(
            StatusCode::CONFLICT,
            "key_taken",
            "this key already has a device",
        ),
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
) -> Result<Cbor<TokenResponse>, Problem> {
    if !authed.is_admin() {
        return Err(Problem::forbidden("only an administrator adds a device"));
    }
    let max = app.limits.enrollment_ttl_max;
    let ttl = req
        .expires_in
        .map_or(Duration::from_secs(24 * 3600), Duration::from_secs)
        .min(max)
        .max(Duration::from_secs(1));
    let token = new_token();
    app.store
        .add_token(
            token_hash(&token),
            crate::db::TokenKind::Device,
            Some(authed.device.team),
            Some(now() + ttl.as_secs() as i64),
            now(),
        )
        .await
        .map_err(store_problem)?;
    Ok(Cbor(TokenResponse {
        token,
        expires_in: ttl.as_secs(),
    }))
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
        (Stream::Team, Kind::Commit, Some(_)) | (_, Kind::Message, None) => {}
        (Stream::Team, Kind::Commit, None) => {
            return Err(Problem::bad_request("no_epoch", "a commit says its epoch"))
        }
        (Stream::Project(_), Kind::Commit, _) => {
            return Err(Problem::bad_request(
                "no_commit_here",
                "a project's stream holds no commit",
            ))
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
    Ok(Cbor(Entries { entries, more }))
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
    Ok(Cbor(Entries { entries, more }))
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
