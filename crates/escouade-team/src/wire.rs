//! The server's protocol, version 1, as the client speaks it (`crates/escouade-team-server/README.md`): the bodies of
//! `escouade_team_server::api::wire`, the entries of its streams and the frames of `/v1/live`, in CBOR. The client
//! does not depend on the server's crate: these are the same shapes, and the tests against the real server check
//! that they stay so.
//!
//! What the client writes in the team's stream, the server does not read: [`TeamMessage`], beside the MLS commits.

use escouade_team_crypto::anchor::AnchorGrant;
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
    /// A live connection of it is open now (TS-15). Absent from a server of before: false.
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
    /// A write of a ticket's holder (TS-6): the claim's opaque id and its token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fence: Option<FenceWire>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FenceWire {
    #[serde(with = "serde_bytes")]
    pub ticket: Vec<u8>,
    pub token: u64,
}

/// Who has access to a project on the server (TS-6).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessWire {
    pub everyone: bool,
    #[serde(default)]
    pub devices: Vec<serde_bytes::ByteBuf>,
}

/// The claim of a ticket, as the server keeps it (TS-6): its opaque id, the device that holds it, until when, under
/// which token. Seconds of the server's clock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    #[serde(with = "serde_bytes")]
    pub ticket: Vec<u8>,
    #[serde(default, with = "opt_bytes", skip_serializing_if = "Option::is_none")]
    pub holder: Option<Vec<u8>>,
    pub until: i64,
    pub token: u64,
    pub version: u64,
    pub claimed_at: i64,
    pub renewed_at: i64,
    #[serde(default, with = "opt_bytes", skip_serializing_if = "Option::is_none")]
    pub last_holder: Option<Vec<u8>>,
    /// The device that handed it to its holder (TS-17), while the holder has not taken it yet.
    #[serde(default, with = "opt_bytes", skip_serializing_if = "Option::is_none")]
    pub handed_by: Option<Vec<u8>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ClaimRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimToken {
    pub token: u64,
}

/// Hands a claim to another device of the project (TS-17), under its token.
#[derive(Debug, Serialize, Deserialize)]
pub struct HandRequest {
    pub token: u64,
    #[serde(with = "serde_bytes")]
    pub to: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub claims: Vec<Claim>,
    pub now: i64,
    pub lease: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClaimAnswer {
    pub claim: Claim,
    pub now: i64,
    pub lease: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyNumber {
    pub n: u64,
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
    /// A project's stream: the number its last snapshot holds up to, when it is after `since` (start from the
    /// snapshot: its messages may be gone, after the retention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<u64>,
    /// A project's stream: the number its last snapshot holds up to, whatever `since` (SR2-01): a device past it
    /// checks it once all the same, and flags it when it does not open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest: Option<u64>,
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

/// The snapshot a device found unreadable (SR2-01).
#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotFlag {
    pub upto: u64,
}

/// A snapshot a device found unreadable, as the team's administrators are told (SR2-01).
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// A `claim` frame (TS-6): a ticket taken, renewed or given back.
    #[serde(default)]
    pub claim: Option<Claim>,
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
    /// The anchor of the recovery or of the escrow wraps (SR2-09), sealed by an administrator's device for another
    /// administrator's device, to the X25519 form of its leaf key (`escouade_team_crypto::anchor`).
    Anchor(AnchorGrant),
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
    use escouade_team_crypto::recovery::{RecoveryWrap, Sealed, WrapSignatures};

    fn signed() -> WrapSignatures {
        WrapSignatures {
            team: [3; 32],
            signer: vec![4; 32],
            signature: vec![5; 64],
            anchor: vec![6; 64],
        }
    }

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
                signed: signed(),
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
            signed: signed(),
        });
        assert!(
            matches!(decode::<TeamMessage>(&encode(&long)).unwrap(), TeamMessage::Escrow(w) if w.sealed.ciphertext.len() == 20_000)
        );
        let grant = TeamMessage::Anchor(AnchorGrant {
            device: [8; 16],
            anchor: vec![9; 32],
            sealed: Sealed {
                kem_output: vec![1; 32],
                ciphertext: vec![2; 48],
            },
        });
        assert!(
            matches!(decode::<TeamMessage>(&encode(&grant)).unwrap(), TeamMessage::Anchor(g) if g.device == [8; 16] && g.anchor == vec![9; 32])
        );
        let g = TeamMessage::TokenGranted { hash: [7; 32] };
        assert!(
            matches!(decode::<TeamMessage>(&encode(&g)).unwrap(), TeamMessage::TokenGranted { hash } if hash == [7; 32])
        );
    }
}
