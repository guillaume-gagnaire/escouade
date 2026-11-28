//! The routes of the API, version 1. Every one but `/healthz`, the enrolment by a token and the two steps of the
//! signed challenge asks for a session (`auth::Authed`).

use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::auth::{self, Authed};
use crate::db::{
    Claim, ClaimAsk, Entry, Fence, Kind, NewEntry, Page, Quotas, Role, Snapshot, StoreError, Stream,
};
use crate::live::Event;
use crate::metrics::Metrics;
use crate::transport::{body, Cbor, Problem};
use crate::util::{id_from, id_from_hex, new_token, now, token_hash, Id};
use crate::{ClientIp, Shared};
use escouade_audit::event::SessionRefusal;

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

    /// The token of a try (SR2-05): what takes the answer.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct PairTried {
        pub token: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PairTake {
        pub token: String,
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
        /// A live connection of it is open now (TS-15: « sur combien de postes en ligne »). Absent from a server of
        /// before: false.
        #[serde(default)]
        pub online: bool,
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
        /// A write of a ticket's holder (TS-6): the claim's opaque id and the token it holds it under. Refused (409
        /// `stale_token`) when the token is no longer the current one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub fence: Option<FenceWire>,
        /// A commit says, in clear, that it changes its member's own devices only (SR2-02): a member's commit is
        /// refused without it (403). The devices check the same, signed inside the commit.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        pub own: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct FenceWire {
        #[serde(with = "serde_bytes")]
        pub ticket: Vec<u8>,
        pub token: u64,
    }

    /// Who has access to a project (TS-6): every device of the team, or those listed.
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct AccessWire {
        pub everyone: bool,
        #[serde(default)]
        pub devices: Vec<serde_bytes::ByteBuf>,
    }

    /// Takes a ticket as it was at `expected` (its claim's version), when given.
    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct ClaimRequest {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expected: Option<u64>,
    }

    /// Renews or gives back a claim, under its token.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TokenRequest {
        pub token: u64,
    }

    /// Hands a claim to another device of the project (TS-17), under its token.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct HandRequest {
        pub token: u64,
        #[serde(with = "serde_bytes")]
        pub to: Vec<u8>,
    }

    /// A project's claims, and the server's time (seconds), which the leases are counted in.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Claims {
        pub claims: Vec<Claim>,
        pub now: i64,
        /// The length of a lease (seconds).
        pub lease: i64,
    }

    /// A claim, with the server's time.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ClaimAnswer {
        pub claim: Claim,
        pub now: i64,
        pub lease: i64,
    }

    /// The next number of a project's tickets.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct KeyNumber {
        pub n: u64,
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
        /// A project's stream: the number its last snapshot holds up to, whatever `since` (SR2-01): a device past it
        /// checks it once all the same, and flags it when it does not open.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub latest: Option<u64>,
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
        /// The messages of the stream forgotten at once: none since SR2-01 (they stay for the retention, then go
        /// when no device flagged the snapshot).
        pub pruned: u64,
    }

    /// The snapshot a device found unreadable (SR2-01).
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SnapshotFlag {
        pub upto: u64,
    }

    /// A snapshot a device found unreadable, as the team's administrators are told (SR2-01).
    #[derive(Debug, Serialize, Deserialize)]
    pub struct UnreadableInfo {
        #[serde(with = "serde_bytes")]
        pub project: Vec<u8>,
        pub upto: u64,
        /// The device that posted it, and when.
        #[serde(with = "serde_bytes")]
        pub device: Vec<u8>,
        pub at: i64,
        /// The first device that found it unreadable, and when.
        #[serde(with = "serde_bytes")]
        pub flagged_by: Vec<u8>,
        pub flagged_at: i64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct UnreadableSnapshots {
        pub snapshots: Vec<UnreadableInfo>,
    }

    /// An entry the server's operator removed from a stream (`drop-entry`, SR2-02), as the devices are told: what
    /// tells the hole from a rollback. Never its bytes, nor who removed it (that stays in the server's audit).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DroppedInfo {
        /// The project's stream, None for the team's.
        #[serde(with = "serde_bytes", default, skip_serializing_if = "Option::is_none")]
        pub project: Option<Vec<u8>>,
        pub seq: u64,
        pub kind: Kind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub epoch: Option<u64>,
        /// The device that wrote it, and when.
        #[serde(with = "serde_bytes")]
        pub device: Vec<u8>,
        pub written_at: i64,
        /// The SHA-256 of the entry as it was served (its CBOR).
        #[serde(with = "serde_bytes")]
        pub digest: Vec<u8>,
        /// When the operator removed it.
        pub at: i64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DroppedEntries {
        pub dropped: Vec<DroppedInfo>,
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
        StoreError::TokenUsed(_) => Problem::new(
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
        StoreError::Drop(why) => Problem::new(StatusCode::CONFLICT, "drop_refused", why),
        StoreError::Reserved => Problem::new(
            StatusCode::CONFLICT,
            "epoch_reserved",
            "the commit of this epoch was declared null: the epoch is kept to the administrators until their next commit",
        ),
        StoreError::Void(why) => Problem::new(StatusCode::CONFLICT, "void_refused", why),
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
        StoreError::NoAccess => Problem::new(
            StatusCode::FORBIDDEN,
            "no_access",
            "this device has no access to this project",
        ),
        StoreError::NotAdmin => Problem::new(
            StatusCode::FORBIDDEN,
            "not_admin",
            "only an administrator changes who has access to a project",
        ),
        StoreError::Taken(c) => Problem::new(
            StatusCode::CONFLICT,
            "taken",
            "another device holds this ticket",
        )
        .with_detail(&c),
        StoreError::StaleVersion(c) => Problem::new(
            StatusCode::CONFLICT,
            "stale_version",
            "the ticket changed since the version expected",
        )
        .with_detail(&c),
        StoreError::ClaimLost(c) => Problem::new(
            StatusCode::CONFLICT,
            "claim_lost",
            "this claim is no longer this device's",
        )
        .with_detail(&c),
        StoreError::StaleToken(c) => Problem::new(
            StatusCode::CONFLICT,
            "stale_token",
            "a write under a claim token that is no longer the current one",
        )
        .with_detail(&c),
        StoreError::NotInProject(c) => Problem::new(
            StatusCode::CONFLICT,
            "not_in_project",
            "the device a ticket is handed to is not one of this project",
        )
        .with_detail(&c),
        StoreError::Backend(e) => {
            tracing::error!("base: {e}");
            Problem::internal()
        }
    }
}

/// Whether the device may read and write the project (TS-6): one the server never heard of is open to the team.
async fn may(app: &Shared, authed: &Authed, project: &Id) -> Result<(), Problem> {
    let access = app
        .store
        .access(authed.device.team, *project)
        .await
        .map_err(store_problem)?;
    if access.is_some_and(|a| !a.admits(&authed.device.id)) {
        Metrics::inc(&app.metrics.refused_access);
        return Err(store_problem(StoreError::NoAccess));
    }
    Ok(())
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
    ClientIp(ip): ClientIp,
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
    let hash = token_hash(&req.token);
    let (device, kind) = app
        .store
        .enroll(hash, public_key, now(), app.quotas())
        .await
        .map_err(|e| {
            match &e {
                StoreError::BadToken => Metrics::inc(&app.metrics.auth_failures),
                StoreError::TokenUsed(team) => app.audit.token_reused(*team, &hash, ip),
                _ => {}
            }
            store_problem(e)
        })?;
    app.audit.enrolled(&device, kind, ip);
    Ok(Cbor(EnrollResponse {
        team: device.team.to_vec(),
        device: device.id.to_vec(),
        role: device.role,
    }))
}

pub async fn challenge(
    State(app): State<Shared>,
    ClientIp(ip): ClientIp,
    Cbor(req): Cbor<ChallengeRequest>,
) -> Result<Cbor<ChallengeResponse>, Problem> {
    let id = device_id(&req.device)?;
    let known = app.store.device(id).await.map_err(store_problem)?;
    let Some(device) = known.clone().filter(|d| d.revoked_at.is_none()) else {
        Metrics::inc(&app.metrics.auth_failures);
        let (team, cause) = match known {
            Some(d) => (Some(d.team), SessionRefusal::Revoked),
            None => (None, SessionRefusal::Unknown),
        };
        app.audit.session_refused(team, &id, cause, ip);
        return Err(Problem::unauthorized());
    };
    let ttl = app.limits.challenge_ttl;
    let challenge = app
        .sessions
        .challenge(id, device.team, Instant::now() + ttl)?;
    Ok(Cbor(ChallengeResponse {
        challenge: challenge.to_vec(),
        expires_in: ttl.as_secs(),
    }))
}

pub async fn session(
    State(app): State<Shared>,
    ClientIp(ip): ClientIp,
    Cbor(req): Cbor<SessionRequest>,
) -> Result<Cbor<TokenResponse>, Problem> {
    let id = device_id(&req.device)?;
    let known = app.store.device(id).await.map_err(store_problem)?;
    let refuse = |cause: SessionRefusal| {
        Metrics::inc(&app.metrics.auth_failures);
        let cause = match &known {
            None => SessionRefusal::Unknown,
            Some(d) if d.revoked_at.is_some() => SessionRefusal::Revoked,
            Some(_) => cause,
        };
        app.audit
            .session_refused(known.as_ref().map(|d| d.team), &id, cause, ip);
        Problem::unauthorized()
    };
    let challenge: [u8; 32] = req
        .challenge
        .as_slice()
        .try_into()
        .map_err(|_| refuse(SessionRefusal::BadSignature))?;
    if !app.sessions.take_challenge(&id, &challenge) {
        return Err(refuse(SessionRefusal::BadSignature));
    }
    let device = app
        .store
        .device(id)
        .await
        .map_err(store_problem)?
        .filter(|d| d.revoked_at.is_none())
        .ok_or_else(|| refuse(SessionRefusal::Revoked))?;
    if !auth::verify(&device.public_key, &id, &challenge, &req.signature) {
        return Err(refuse(SessionRefusal::BadSignature));
    }
    let ttl = app.limits.session_ttl;
    let token = app.sessions.open(id, Instant::now() + ttl)?;
    app.audit.session_opened(&device, ip);
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
    // A machine of the team stays one (OE-1, as the roster's rules): it leaves, another is enrolled.
    let current = app.store.device(device).await.map_err(store_problem)?;
    if current
        .as_ref()
        .is_some_and(|d| d.team == authed.device.team && d.role == Role::Service)
        && req.role != Role::Service
    {
        return Err(Problem::new(
            StatusCode::CONFLICT,
            "service_stays",
            "a machine of the team stays one: remove it, and enroll another",
        ));
    }
    if app
        .store
        .set_role(authed.device.team, device, req.role)
        .await
        .map_err(store_problem)?
    {
        if let Some(before) = current {
            app.audit
                .role_changed(authed.by(), &device, before.role, req.role);
        }
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
                online: app.hub.is_live(&d.id),
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
    app.audit.revoked(authed.by(), &device);
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
) -> Result<Cbor<PairTried>, Problem> {
    bounded(&req.message, crate::pair::MAX_MESSAGE)?;
    bounded(&req.confirm, crate::pair::MAX_MESSAGE)?;
    let token = app
        .pairs
        .try_once(&slot, req.message, req.confirm)
        .map_err(pair_problem)?;
    Ok(Cbor(PairTried { token }))
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

/// The new device takes the answer with the token of its try (SR2-05).
pub async fn pair_take(
    State(app): State<Shared>,
    Path(slot): Path<String>,
    Cbor(req): Cbor<PairTake>,
) -> Result<Response, Problem> {
    if req.token.len() > 2 * crate::pair::TOKEN_BYTES {
        return Err(pair_problem(crate::pair::PairError::NotYours));
    }
    match app
        .pairs
        .take_answer(&slot, &req.token)
        .map_err(pair_problem)?
    {
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

/// `/join/<id>`: an invitation link of before (`https://<server>/join/<id>#…`) opened in a browser. A page of this
/// server never touches an invitation's secret (SR2-08): a script of its own could read what follows the `#`
/// (`location.hash`), and a server taken over would then enter in the invitee's place. The links are now the app's
/// (`escouade://team/join/<server>/<id>#…`); this page is a fixed text that says to ask for a new one: no script
/// (the policy forbids any), no resource from elsewhere, nothing read of the link.
pub async fn join_page(Path(id): Path<String>) -> Result<Response, Problem> {
    path_id(&id)?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(JOIN_POLICY),
            ),
            (
                header::REFERRER_POLICY,
                HeaderValue::from_static("no-referrer"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        JOIN_PAGE,
    )
        .into_response())
}

const JOIN_PAGE: &str = include_str!("join.html");

/// No script at all, no resource from elsewhere.
const JOIN_POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'; script-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// The stream of the team's list of projects (`escouade_team::catalog::CATALOG_STREAM`): its content is encrypted,
/// only its writers are known here.
pub const CATALOG_STREAM: Id = *b"escouade/project";

/// The stream of the team's circuit breakers (`escouade_team::breaker::BREAKER_STREAM`, TS-15): its administrators'
/// too. The clients judge each entry's signature and its author's role; the server, which knows the roles, does not
/// keep a member's (defense in depth).
pub const BREAKER_STREAM: Id = *b"escouade/breaker";

/// The streams only an administrator writes.
pub const ADMIN_STREAMS: [Id; 2] = [CATALOG_STREAM, BREAKER_STREAM];

/// The stream of the administrators' key of the costs (`escouade_team::costs::KEY_STREAM`, TS-16): its public half and
/// its wraps for the administrators' devices, written by an administrator alone.
pub const COST_KEY_STREAM: Id = *b"escouade/costkey";

/// The stream of the totals of the costs (`escouade_team::costs::TOTALS_STREAM`, TS-16): every member writes there
/// what its device spent, sealed to the administrators' key.
pub const COST_TOTALS_STREAM: Id = *b"escouade/costsum";

/// The stream of the team's private store of plugins (`escouade_team::plugins::PLUGINS_STREAM`, TS-14): every member
/// proposes there, the devices refuse what an administrator alone writes from anyone else.
pub const PLUGINS_STREAM: Id = *b"escouade/plugins";

/// The streams of the team itself (its list of projects, its settings, the proposals of its members, TS-7, the costs,
/// TS-16, its private store, TS-14, the circuit breakers, TS-15): open to every device of the team, always. Nobody restricts
/// them, or a member could shut the administrators out of them.
pub const TEAM_STREAMS: [Id; 7] = [
    CATALOG_STREAM,
    *b"escouade/setting",
    *b"escouade/propose",
    COST_KEY_STREAM,
    COST_TOTALS_STREAM,
    PLUGINS_STREAM,
    BREAKER_STREAM,
];

/// The team's streams a machine of the team never writes (OE-1): all of them but the totals of the costs, where it
/// says what it spent as every device does.
pub const SERVICE_DENIED_STREAMS: [Id; 6] = [
    CATALOG_STREAM,
    *b"escouade/setting",
    *b"escouade/propose",
    COST_KEY_STREAM,
    PLUGINS_STREAM,
    BREAKER_STREAM,
];

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
        (_, Kind::Commit | Kind::Void, Some(_)) | (_, Kind::Message, None) => {}
        (_, Kind::Commit | Kind::Void, None) => {
            return Err(Problem::bad_request(
                "no_epoch",
                "a commit, or its void, says its epoch",
            ))
        }
        (_, Kind::Message, Some(_)) => {
            return Err(Problem::bad_request(
                "epoch_on_message",
                "only a commit has an epoch",
            ))
        }
    }
    // SR2-02, in defense in depth: the commits are the administrators', but for a member's change of their own devices,
    // which says so; and only an administrator declares a commit null. The devices check the same, signed.
    match req.kind {
        Kind::Commit if !req.own && !authed.is_admin() => {
            return Err(Problem::forbidden(
                "a member's commit changes their own devices only, and says so",
            ))
        }
        Kind::Void if !authed.is_admin() => {
            return Err(Problem::forbidden(
                "only an administrator declares a commit null",
            ))
        }
        _ => {}
    }
    let team = authed.device.team;
    let size = req.payload.len() as u64;
    let fence = match (&stream, req.fence) {
        (_, None) => None,
        (Stream::Team, Some(_)) => {
            return Err(Problem::bad_request(
                "fence_on_team",
                "only a project's write carries a claim",
            ))
        }
        (Stream::Project(_), Some(f)) => Some(Fence {
            ticket: device_id(&f.ticket)?,
            token: f.token,
        }),
    };
    let entry = NewEntry {
        kind: req.kind,
        epoch: req.epoch,
        device: authed.device.id,
        at: now(),
        payload: req.payload,
        fence,
        by_admin: authed.is_admin(),
    };
    let (entry, epoch) = app
        .store
        .append(team, stream, entry, app.quotas(), app.page(None))
        .await
        .map_err(|e| {
            match (&e, stream, fence) {
                (StoreError::EpochTaken { .. }, _, _) => Metrics::inc(&app.metrics.epoch_conflicts),
                (StoreError::StaleToken(c), Stream::Project(p), Some(f)) => {
                    app.audit
                        .fenced(authed.by(), &p, &f.ticket, f.token, c.token)
                }
                _ => {}
            }
            store_problem(e)
        })?;
    app.audit.written(authed.by(), stream, &entry, epoch);
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
    may(&app, &authed, &project).await?;
    // The team's list of projects (TS-8) and its circuit breakers (TS-15) are its administrators': the clients refuse
    // a member's entry at the reading, and the server, which knows the roles, does not even keep one (defense in
    // depth).
    if ADMIN_STREAMS.contains(&project) && !authed.is_admin() {
        return Err(Problem::forbidden(if project == CATALOG_STREAM {
            "only an administrator writes the team's list of projects"
        } else {
            "only an administrator sets or lifts a circuit breaker"
        }));
    }
    // The administrators' key of the costs (TS-16) likewise: a member who wrote one would have the totals sealed to a
    // key of its own (the clients refuse it at the reading too).
    if project == COST_KEY_STREAM && !authed.is_admin() {
        return Err(Problem::forbidden(
            "only an administrator writes the key of the costs",
        ));
    }
    // A machine of the team (OE-1) writes the content of the projects opened to it, never in the team's own streams:
    // no setting, no list of projects, no proposal, no key, no private plugin, no circuit breaker (the devices refuse
    // it at the reading too, `policy::may_sign`).
    if authed.is_service() && SERVICE_DENIED_STREAMS.contains(&project) {
        return Err(Problem::forbidden(
            "a machine of the team signs nothing of the team's own streams",
        ));
    }
    let appended = append(&app, &authed, Stream::Project(project), req).await?;
    // Its first write makes a project the server never heard of open to the whole team: from then on, only an
    // administrator restricts it.
    app.store
        .open_access(authed.device.team, project)
        .await
        .map_err(store_problem)?;
    Ok(appended)
}

/// Who has access to a project (TS-6).
pub async fn get_access(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
) -> Result<Cbor<AccessWire>, Problem> {
    let project = path_id(&project)?;
    may(&app, &authed, &project).await?;
    let access = app
        .store
        .access(authed.device.team, project)
        .await
        .map_err(store_problem)?
        .unwrap_or(crate::db::Access {
            everyone: true,
            devices: Vec::new(),
        });
    Ok(Cbor(AccessWire {
        everyone: access.everyone,
        devices: access
            .devices
            .iter()
            .map(|d| serde_bytes::ByteBuf::from(d.to_vec()))
            .collect(),
    }))
}

/// Sets who has access to a project: any device of the team for a project the server does not know yet (its
/// creation), an administrator afterwards. Every device listed is one of the team's.
pub async fn put_access(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Cbor(req): Cbor<AccessWire>,
) -> Result<StatusCode, Problem> {
    let project = path_id(&project)?;
    if TEAM_STREAMS.contains(&project) {
        return Err(Problem::forbidden(
            "the team's own streams are the whole team's",
        ));
    }
    if req.devices.len() > app.limits.max_devices as usize {
        return Err(Problem::bad_request("too_many", "too many devices"));
    }
    let team_devices: Vec<Id> = app
        .store
        .devices(authed.device.team)
        .await
        .map_err(store_problem)?
        .into_iter()
        .map(|d| d.id)
        .collect();
    let mut devices = Vec::new();
    for d in &req.devices {
        let id = device_id(d)?;
        if !team_devices.contains(&id) {
            return Err(Problem::bad_request(
                "unknown_device",
                "a device listed is not of this team",
            ));
        }
        // Each device once (SR2-04): the list is the team's, not a place to fill.
        if devices.contains(&id) {
            return Err(Problem::bad_request(
                "duplicate_device",
                "a device is listed twice",
            ));
        }
        devices.push(id);
    }
    app.store
        .set_access(
            authed.device.team,
            project,
            crate::db::Access {
                everyone: req.everyone,
                devices: devices.clone(),
            },
            authed.is_admin(),
            app.quotas(),
        )
        .await
        .map_err(store_problem)?;
    app.audit
        .access_changed(authed.by(), &project, req.everyone, &devices);
    Ok(StatusCode::NO_CONTENT)
}

fn claim_lease(app: &Shared) -> i64 {
    app.limits.claim_lease.as_secs().max(1) as i64
}

async fn claim_ask(
    app: &Shared,
    authed: &Authed,
    project: &str,
    ticket: &str,
    ask: ClaimAsk,
) -> Result<Cbor<ClaimAnswer>, Problem> {
    let project = path_id(project)?;
    let ticket = path_id(ticket)?;
    may(app, authed, &project).await?;
    let lease = match ask {
        ClaimAsk::Hand { .. } => app.limits.hand_lease.as_secs().max(1) as i64,
        _ => claim_lease(app),
    };
    let at = now();
    let (claim, before) = app
        .store
        .claim(
            authed.device.team,
            project,
            ticket,
            authed.device.id,
            ask,
            at,
            lease,
            app.quotas(),
        )
        .await
        .map_err(|e| {
            if matches!(e, StoreError::Taken(_)) {
                Metrics::inc(&app.metrics.claims_refused);
            }
            store_problem(e)
        })?;
    if !matches!(ask, ClaimAsk::Renew { .. }) {
        Metrics::inc(&app.metrics.claims);
    }
    app.audit
        .claimed(authed.by(), &project, ask, &claim, before.as_ref());
    app.hub
        .publish(&authed.device.team, Event::Claim(project, claim.clone()));
    Ok(Cbor(ClaimAnswer {
        claim,
        now: at,
        lease,
    }))
}

/// Takes a ticket (§4.2): one demand passes, the others are told who holds it (409 `taken`, the claim as detail).
pub async fn take_claim(
    State(app): State<Shared>,
    authed: Authed,
    Path((project, ticket)): Path<(String, String)>,
    Cbor(req): Cbor<ClaimRequest>,
) -> Result<Cbor<ClaimAnswer>, Problem> {
    claim_ask(
        &app,
        &authed,
        &project,
        &ticket,
        ClaimAsk::Take {
            expected: req.expected,
        },
    )
    .await
}

/// Renews a lease (every 3 minutes while the agent runs).
pub async fn renew_claim(
    State(app): State<Shared>,
    authed: Authed,
    Path((project, ticket)): Path<(String, String)>,
    Cbor(req): Cbor<TokenRequest>,
) -> Result<Cbor<ClaimAnswer>, Problem> {
    claim_ask(
        &app,
        &authed,
        &project,
        &ticket,
        ClaimAsk::Renew { token: req.token },
    )
    .await
}

/// Gives a ticket back (its agent stopped, or it went « À tester »).
pub async fn release_claim(
    State(app): State<Shared>,
    authed: Authed,
    Path((project, ticket)): Path<(String, String)>,
    Cbor(req): Cbor<TokenRequest>,
) -> Result<Cbor<ClaimAnswer>, Problem> {
    claim_ask(
        &app,
        &authed,
        &project,
        &ticket,
        ClaimAsk::Release { token: req.token },
    )
    .await
}

/// Hands a ticket to another device of the project (TS-17, §11): it holds it at once, under a new token, until it
/// takes it or gives it back; 409 `not_in_project` for a device revoked, without access, or the holder itself.
pub async fn hand_claim(
    State(app): State<Shared>,
    authed: Authed,
    Path((project, ticket)): Path<(String, String)>,
    Cbor(req): Cbor<HandRequest>,
) -> Result<Cbor<ClaimAnswer>, Problem> {
    let to: Id = req
        .to
        .as_slice()
        .try_into()
        .map_err(|_| Problem::bad_request("bad_device", "a device id is 16 bytes"))?;
    claim_ask(
        &app,
        &authed,
        &project,
        &ticket,
        ClaimAsk::Hand {
            token: req.token,
            to,
        },
    )
    .await
}

/// Takes back a ticket this device handed, while its new holder has not taken it (the passage abandoned).
pub async fn withdraw_claim(
    State(app): State<Shared>,
    authed: Authed,
    Path((project, ticket)): Path<(String, String)>,
    Cbor(req): Cbor<TokenRequest>,
) -> Result<Cbor<ClaimAnswer>, Problem> {
    claim_ask(
        &app,
        &authed,
        &project,
        &ticket,
        ClaimAsk::Withdraw { token: req.token },
    )
    .await
}

/// A project's claims, with the server's time.
pub async fn list_claims(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
) -> Result<Cbor<Claims>, Problem> {
    let project = path_id(&project)?;
    may(&app, &authed, &project).await?;
    let claims = app
        .store
        .claims(authed.device.team, project)
        .await
        .map_err(store_problem)?;
    Ok(Cbor(Claims {
        claims,
        now: now(),
        lease: claim_lease(&app),
    }))
}

/// The next number of a project's tickets (§4.1): the server sees the counter alone, the prefix is encrypted and the
/// device forms the key.
pub async fn next_key(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
) -> Result<Cbor<KeyNumber>, Problem> {
    let project = path_id(&project)?;
    may(&app, &authed, &project).await?;
    let n = app
        .store
        .next_key(authed.device.team, project, app.quotas())
        .await
        .map_err(store_problem)?;
    Ok(Cbor(KeyNumber { n }))
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
        latest: None,
    }))
}

pub async fn read_project(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Query(q): Query<Since>,
) -> Result<Cbor<Entries>, Problem> {
    let project = path_id(&project)?;
    may(&app, &authed, &project).await?;
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
    let latest = app
        .store
        .snapshot_upto(authed.device.team, project)
        .await
        .map_err(store_problem)?;
    Ok(Cbor(Entries {
        entries,
        more,
        snapshot: latest.filter(|upto| *upto > q.since),
        latest,
    }))
}

/// Keeps a project's snapshot (TS-5) beside the ones before. Its messages up to `upto` stay for the retention
/// (SR2-01, `Limits::snapshot_retention`): the sweep forgets them after it, unless a device flagged the snapshot
/// unreadable. A snapshot not after the last one kept, or beyond the stream, is refused (409 `snapshot_refused`); one of
/// the team's own streams too (403): their entries are signed by their writers, no device reads a snapshot of them.
pub async fn put_snapshot(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Cbor(req): Cbor<SnapshotPut>,
) -> Result<Cbor<SnapshotKept>, Problem> {
    let project = path_id(&project)?;
    if TEAM_STREAMS.contains(&project) {
        return Err(Problem::forbidden(
            "the team's own streams take no snapshot: their entries stay",
        ));
    }
    may(&app, &authed, &project).await?;
    if req.payload.is_empty() {
        return Err(Problem::bad_request("empty", "a snapshot carries bytes"));
    }
    if req.payload.len() > app.limits.max_file {
        return Err(Problem::too_large(app.limits.max_file));
    }
    app.store
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
    Ok(Cbor(SnapshotKept { pruned: 0 }))
}

#[derive(Debug, Deserialize)]
pub struct Before {
    #[serde(default)]
    pub before: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Upto {
    pub upto: u64,
}

/// A project's last snapshot, or with `?before=n` its last one before `n` (a device that found the last one
/// unreadable goes back to the one before, SR2-01).
pub async fn get_snapshot(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Query(q): Query<Before>,
) -> Result<Cbor<SnapshotInfo>, Problem> {
    let project = path_id(&project)?;
    may(&app, &authed, &project).await?;
    let s = app
        .store
        .snapshot(authed.device.team, project, q.before)
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

/// A device of the project found its snapshot at `upto` unreadable (SR2-01): the snapshot is flagged, its messages are
/// never pruned while it is, and the team's administrators are told (`unreadable_snapshots`). 404 without such a
/// snapshot.
pub async fn flag_snapshot(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Cbor(req): Cbor<SnapshotFlag>,
) -> Result<StatusCode, Problem> {
    let project = path_id(&project)?;
    may(&app, &authed, &project).await?;
    let found = app
        .store
        .flag_snapshot(
            authed.device.team,
            project,
            req.upto,
            authed.device.id,
            now(),
        )
        .await
        .map_err(store_problem)?;
    if !found {
        return Err(Problem::not_found());
    }
    Metrics::inc(&app.metrics.unreadable_snapshots);
    Ok(StatusCode::NO_CONTENT)
}

/// An administrator removes a project's snapshot at `?upto=n` (SR2-01): the one before, and the messages after it,
/// take its place. 404 without such a snapshot; 409 `snapshot_refused` for the one the stream was pruned to (its
/// messages are gone: nothing would take its place).
pub async fn remove_snapshot(
    State(app): State<Shared>,
    authed: Authed,
    Path(project): Path<String>,
    Query(q): Query<Upto>,
) -> Result<StatusCode, Problem> {
    let project = path_id(&project)?;
    if !authed.is_admin() {
        return Err(Problem::forbidden(
            "only an administrator removes a snapshot",
        ));
    }
    let found = app
        .store
        .remove_snapshot(authed.device.team, project, q.upto)
        .await
        .map_err(store_problem)?;
    if !found {
        return Err(Problem::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

/// The entries the server's operator removed from the team's streams (`drop-entry`, SR2-02): every device of the team
/// reads them, to take the hole for what it is rather than a rollback, and its administrators are told. A project's
/// entries are listed to the devices that have access to it.
pub async fn dropped(
    State(app): State<Shared>,
    authed: Authed,
) -> Result<Cbor<DroppedEntries>, Problem> {
    let team = authed.device.team;
    let all = app.store.dropped(team).await.map_err(store_problem)?;
    let mut dropped = Vec::with_capacity(all.len());
    for d in all {
        if let Some(project) = d.project {
            let access = app
                .store
                .access(team, project)
                .await
                .map_err(store_problem)?;
            if access.is_some_and(|a| !a.admits(&authed.device.id)) {
                continue;
            }
        }
        dropped.push(DroppedInfo {
            project: d.project.map(|p| p.to_vec()),
            seq: d.seq,
            kind: d.kind,
            epoch: d.epoch,
            device: d.device.to_vec(),
            written_at: d.written_at,
            digest: d.digest.to_vec(),
            at: d.at,
        });
    }
    Ok(Cbor(DroppedEntries { dropped }))
}

/// The team's snapshots a device found unreadable, each with the device that posted it (SR2-01): for its
/// administrators.
pub async fn unreadable_snapshots(
    State(app): State<Shared>,
    authed: Authed,
) -> Result<Cbor<UnreadableSnapshots>, Problem> {
    if !authed.is_admin() {
        return Err(Problem::forbidden(
            "only an administrator reads the unreadable snapshots",
        ));
    }
    let list = app
        .store
        .unreadable_snapshots(authed.device.team)
        .await
        .map_err(store_problem)?;
    Ok(Cbor(UnreadableSnapshots {
        snapshots: list
            .into_iter()
            .map(|u| UnreadableInfo {
                project: u.project.to_vec(),
                upto: u.upto,
                device: u.device.to_vec(),
                at: u.at,
                flagged_by: u.flagged_by.to_vec(),
                flagged_at: u.flagged_at,
            })
            .collect(),
    }))
}

// The files (SR2-03). Each is attached to a project at its putting (or to the team, under one of `TEAM_STREAMS`): it
// is read by the devices that have access to that project, as its stream, and deleted by the device that put it or an
// administrator. A file attached to nothing (put by the old route, or before SR2-03) is its author's and the
// administrators' alone, until one of them reads it under a project, which attaches it there.

/// Whether `file` may be read under `project` (None: the old route), attaching it to `project` when it is attached to
/// nothing and the device may: its author or an administrator. A file of another project is not found under this one.
async fn readable(
    app: &Shared,
    authed: &Authed,
    project: Option<Id>,
    file: Id,
) -> Result<(), Problem> {
    let team = authed.device.team;
    if let Some(p) = &project {
        may(app, authed, p).await?;
    }
    let stored = app
        .store
        .file(team, file)
        .await
        .map_err(store_problem)?
        .ok_or_else(Problem::not_found)?;
    match (stored.project, project) {
        (Some(attached), Some(asked)) if attached == asked => Ok(()),
        (Some(_), Some(_)) => Err(Problem::not_found()),
        (Some(attached), None) => may(app, authed, &attached).await,
        (None, asked) => {
            if stored.device != authed.device.id && !authed.is_admin() {
                // Not found rather than refused: the old files stay out of sight of the others until attached.
                return Err(Problem::not_found());
            }
            if let Some(asked) = asked {
                app.store
                    .attach_file(team, file, asked)
                    .await
                    .map_err(store_problem)?;
            }
            Ok(())
        }
    }
}

async fn put_file_in(
    app: Shared,
    authed: Authed,
    project: Option<Id>,
    file: Id,
    req: Request,
) -> Result<StatusCode, Problem> {
    if let Some(p) = &project {
        may(&app, &authed, p).await?;
    }
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
            project,
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

async fn get_file_in(
    app: Shared,
    authed: Authed,
    project: Option<Id>,
    file: Id,
) -> Result<Response, Problem> {
    readable(&app, &authed, project, file).await?;
    let bytes = app
        .blobs
        .get(&authed.device.team, &file)
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

async fn delete_file_in(
    app: Shared,
    authed: Authed,
    project: Option<Id>,
    file: Id,
) -> Result<StatusCode, Problem> {
    let team = authed.device.team;
    let stored = app
        .store
        .file(team, file)
        .await
        .map_err(store_problem)?
        .ok_or_else(Problem::not_found)?;
    if let (Some(attached), Some(asked)) = (stored.project, project) {
        if attached != asked {
            return Err(Problem::not_found());
        }
    }
    if stored.device != authed.device.id && !authed.is_admin() {
        // Seen by the device or not, a file is deleted by its author or an administrator alone: the deletion is final.
        if let Some(attached) = stored.project {
            may(&app, &authed, &attached).await?;
        } else {
            return Err(Problem::not_found());
        }
        Metrics::inc(&app.metrics.refused_access);
        return Err(Problem::forbidden(
            "a file is deleted by the device that put it or an administrator",
        ));
    }
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

/// `PUT /v1/projects/{project}/files/{file}`: a file attached to the project.
pub async fn put_project_file(
    State(app): State<Shared>,
    authed: Authed,
    Path((project, file)): Path<(String, String)>,
    req: Request,
) -> Result<StatusCode, Problem> {
    let (project, file) = (path_id(&project)?, path_id(&file)?);
    put_file_in(app, authed, Some(project), file, req).await
}

pub async fn get_project_file(
    State(app): State<Shared>,
    authed: Authed,
    Path((project, file)): Path<(String, String)>,
) -> Result<Response, Problem> {
    let (project, file) = (path_id(&project)?, path_id(&file)?);
    get_file_in(app, authed, Some(project), file).await
}

pub async fn delete_project_file(
    State(app): State<Shared>,
    authed: Authed,
    Path((project, file)): Path<(String, String)>,
) -> Result<StatusCode, Problem> {
    let (project, file) = (path_id(&project)?, path_id(&file)?);
    delete_file_in(app, authed, Some(project), file).await
}

/// The old route (before SR2-03), kept for one version: a file put here is attached to nothing.
pub async fn put_file(
    State(app): State<Shared>,
    authed: Authed,
    Path(file): Path<String>,
    req: Request,
) -> Result<StatusCode, Problem> {
    let file = path_id(&file)?;
    put_file_in(app, authed, None, file, req).await
}

pub async fn get_file(
    State(app): State<Shared>,
    authed: Authed,
    Path(file): Path<String>,
) -> Result<Response, Problem> {
    let file = path_id(&file)?;
    get_file_in(app, authed, None, file).await
}

pub async fn delete_file(
    State(app): State<Shared>,
    authed: Authed,
    Path(file): Path<String>,
) -> Result<StatusCode, Problem> {
    let file = path_id(&file)?;
    delete_file_in(app, authed, None, file).await
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
