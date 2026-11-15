//! The server's protocol, version 1, as the client speaks it (`crates/escouade-team-server/README.md`): the bodies of
//! `escouade_team_server::api::wire`, the entries of its streams and the frames of `/v1/live`, in CBOR. The client
//! does not depend on the server's crate: these are the same shapes, and the tests against the real server check
//! that they stay so.
//!
//! What the client writes in the team's stream, the server does not read: [`TeamMessage`], beside the MLS commits.

use escouade_team_crypto::content::SealedObject;
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
    /// The SHA-256 of a token this device made (an invitation's, `invite::server_token`): the server registers it.
    #[serde(default, with = "opt_bytes", skip_serializing_if = "Option::is_none")]
    pub token_hash: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Registered {
    pub expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleRequest {
    pub role: Role,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// A project's stream: the number its last snapshot holds up to, when it is after `since` (its messages up to
    /// there are forgotten: start from the snapshot).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<u64>,
}

/// A project's snapshot, as a device sends it (TS-5).
#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotPut {
    pub upto: u64,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

/// A project's snapshot, as the server serves it.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub pruned: u64,
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
    pub fn serialize<S: serde::Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
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

/// What a device writes in the team's stream beside the commits, in CBOR. The server keeps and numbers it, and reads
/// none of it: the links and the wraps are encrypted, a beacon carries an epoch and a hash, a request and a Welcome
/// prove themselves to the device that reads them (`invite.rs`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum TeamMessage {
    History(HistoryLink),
    /// The recovery wrap of an epoch, with the group's id it is sealed for (the holder of the words reads it back
    /// from the stream, with nothing but the words: TS-3).
    Recovery {
        #[serde(with = "serde_bytes")]
        group: Vec<u8>,
        wrap: RecoveryWrap,
    },
    Escrow(EscrowWrap),
    Beacon(Beacon),
    JoinRequest(JoinRequest),
    Welcome {
        #[serde(with = "hex::serde")]
        invitation: [u8; 16],
        delivery: WelcomeDelivery,
    },
    /// A member who is not an administrator on the server asks one to register the token of an invitation for their
    /// own other device (TS-3): sealed as an object of the team, signed by the member's leaf, so that an
    /// administrator knows whose it is. It carries the token's SHA-256 alone, never the invitation's secret.
    TokenRequest(SealedObject),
    /// An administrator registered the token of this SHA-256 with the server.
    TokenGranted {
        #[serde(with = "hex::serde")]
        hash: [u8; 32],
    },
    /// A device's last-resort KeyPackage (TS-5), for the groups of the projects kept to some members: signed by its
    /// leaf, checked against the team's leaves.
    KeyPackage(#[serde(with = "serde_bytes")] Vec<u8>),
    /// The Welcome of the devices added to a project's own group.
    ProjectWelcome {
        #[serde(with = "hex::serde")]
        project: [u8; 16],
        #[serde(with = "serde_bytes")]
        welcome: Vec<u8>,
    },
    /// A history link of a project's own group.
    ProjectHistory {
        #[serde(with = "hex::serde")]
        project: [u8; 16],
        link: HistoryLink,
    },
}

/// What a [`TeamMessage::TokenRequest`] holds, once opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenRequestBody {
    #[serde(with = "hex::serde")]
    pub invitation: [u8; 16],
    #[serde(with = "hex::serde")]
    pub hash: [u8; 32],
    pub expires_in: u64,
    /// What the invitation lets in: the administrator checks that its author may make it.
    pub purpose: escouade_team_crypto::Purpose,
}

pub fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::into_writer(value, &mut out).expect("CBOR encodes into memory");
    out
}

/// Decodes CBOR. Its text strings are read whole: ciborium's scratch buffer (4 KiB by default) would refuse a longer
/// one, and the hexadecimal of a GroupInfo (a recovery wrap) or a Welcome is longer as soon as the team grows. The
/// buffer is the size of what is read, which the server already bounds (an entry, a page).
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let mut scratch = vec![0u8; bytes.len().max(64)];
    ciborium::de::from_reader_with_buffer(bytes, &mut scratch).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use escouade_team_crypto::recovery::{RecoveryWrap, Sealed};

    #[test]
    fn the_team_messages_round_trip() {
        let m = TeamMessage::Recovery {
            group: vec![1, 2, 3],
            wrap: RecoveryWrap {
                epoch: 4,
                sealed: Sealed {
                    kem_output: vec![5; 32],
                    ciphertext: vec![6; 40],
                },
            },
        };
        let back: TeamMessage = decode(&encode(&m)).unwrap();
        assert!(
            matches!(back, TeamMessage::Recovery { group, wrap } if group == vec![1, 2, 3] && wrap.epoch == 4)
        );
        // A long one: the hexadecimal of a GroupInfo is past ciborium's 4 KiB.
        let long = TeamMessage::Escrow(escouade_team_crypto::recovery::EscrowWrap {
            epoch: 1,
            sealed: Sealed {
                kem_output: vec![1; 32],
                ciphertext: vec![2; 20_000],
            },
        });
        assert!(
            matches!(decode::<TeamMessage>(&encode(&long)).unwrap(), TeamMessage::Escrow(w) if w.sealed.ciphertext.len() == 20_000)
        );
        let g = TeamMessage::TokenGranted { hash: [7; 32] };
        assert!(
            matches!(decode::<TeamMessage>(&encode(&g)).unwrap(), TeamMessage::TokenGranted { hash } if hash == [7; 32])
        );
    }
}
