//! The server's protocol, version 1, as the client speaks it (`crates/escouade-team-server/README.md`): the bodies of
//! `escouade_team_server::api::wire`, the entries of its streams and the frames of `/v1/live`, in CBOR. The client
//! does not depend on the server's crate: these are the same shapes, and the tests against the real server check
//! that they stay so.
//!
//! What the client writes in the team's stream, the server does not read: [`TeamMessage`], beside the MLS commits.

use escouade_team_crypto::history::HistoryLink;
use escouade_team_crypto::invite::{JoinRequest, WelcomeDelivery};
use escouade_team_crypto::recovery::{EscrowWrap, RecoveryWrap};
use escouade_team_crypto::Beacon;
use serde::{Deserialize, Serialize};

/// The protocol this client speaks (`Escouade-Protocol`).
pub const PROTOCOL: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Member,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Commit,
    Message,
}

/// One entry of a stream, as the server numbers it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub seq: u64,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub epoch: Option<u64>,
    #[serde(with = "serde_bytes")]
    pub device: Vec<u8>,
    pub at: i64,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochTaken {
    pub expected_epoch: u64,
    pub preceding: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entries {
    pub entries: Vec<Entry>,
    pub more: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub used: u64,
    pub quota: u64,
    pub max_file: u64,
    pub max_entry: u64,
}

/// A refusal: a stable `error`, a message, and a detail for some.
#[derive(Debug, Deserialize)]
pub struct Problem {
    pub error: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub detail: Option<ciborium::Value>,
}

/// A frame of `/v1/live`.
#[derive(Debug, Deserialize)]
pub struct Frame {
    /// `team`, `project` or `lagged`.
    pub stream: String,
    #[serde(default, with = "opt_bytes")]
    pub project: Option<Vec<u8>>,
    #[serde(default)]
    pub entry: Option<Entry>,
}

mod opt_bytes {
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Vec<u8>>, D::Error> {
        let v: Option<serde_bytes::ByteBuf> = serde::Deserialize::deserialize(d)?;
        Ok(v.map(|b| b.into_vec()))
    }
}

/// What a device writes in the team's stream beside the commits, in CBOR. The server keeps and numbers it, and reads
/// none of it: the links and the wraps are encrypted, a beacon carries an epoch and a hash, a request and a Welcome
/// prove themselves to the device that reads them (`invite.rs`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum TeamMessage {
    History(HistoryLink),
    Recovery(RecoveryWrap),
    Escrow(EscrowWrap),
    Beacon(Beacon),
    JoinRequest(JoinRequest),
    Welcome {
        #[serde(with = "hex::serde")]
        invitation: [u8; 16],
        delivery: WelcomeDelivery,
    },
}

pub fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::into_writer(value, &mut out).expect("CBOR encodes into memory");
    out
}

pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    ciborium::from_reader(bytes).map_err(|e| e.to_string())
}
