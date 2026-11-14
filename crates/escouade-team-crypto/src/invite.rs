//! The invitation (C6-A): the link carries an authentication secret only, after the `#`; the keys come from an
//! administrator online, by an MLS Welcome.
//!
//! 1. The administrator's device draws an id (16 bytes) and a secret `s` (32 bytes), and keeps the [`Invitation`]:
//!    role, expiry, single use.
//! 2. The link: `https://<server>/join/<id>#s=<s>&t=<team fingerprint>` (base64url). Nothing after the `#` reaches
//!    the server: neither browsers nor HTTP send it.
//! 3. The invitee's device makes its keys and an MLS KeyPackage, and sends a [`JoinRequest`]: the KeyPackage and
//!    `HMAC-SHA256(s, id ‖ KeyPackage)`. The server, without `s`, cannot put its own KeyPackage in its place.
//! 4. An administrator's device checks the MAC, commits the Add (and the roster's new member), and sends the
//!    Welcome with a confirmation `HMAC-SHA256(s, group ‖ epoch ‖ epoch authenticator)`.
//! 5. The invitee checks that the group it joined has the team fingerprint of its link and the confirmation: the
//!    server cannot show it another team.
//!
//! A link that leaks gives the entry only to the first who uses it, never the content: it holds no key. The
//! legitimate invitee then sees "invitation already used", and the administrator is told.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::identity::MemberId;
use crate::roster::{Role, TeamFingerprint};
use crate::Error;

/// 72 hours.
pub const DEFAULT_LIFETIME: u64 = 72 * 3600;

/// What the invitation lets in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Purpose {
    /// A new member, with this role.
    NewMember(Role),
    /// Another device of a member already there (the QR code of §3.4, the same mechanism).
    OwnDevice(MemberId),
}

/// The invitation as the inviting device keeps it. In the client (TS-3) it is a team object, encrypted and signed,
/// for any administrator device online to honour it.
#[derive(Clone, Debug, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Invitation {
    #[serde(with = "hex::serde")]
    pub id: [u8; 16],
    #[serde(with = "hex::serde")]
    pub secret: [u8; 32],
    #[zeroize(skip)]
    pub purpose: Purpose,
    pub expires_at: u64,
    pub used: bool,
}

/// The link, parsed.
#[derive(Clone, Debug, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct InviteLink {
    pub server: String,
    pub id: [u8; 16],
    pub secret: [u8; 32],
    pub team: [u8; 32],
}

impl InviteLink {
    pub fn to_url(&self) -> String {
        format!(
            "https://{}/join/{}#s={}&t={}",
            self.server,
            hex::encode(self.id),
            URL_SAFE_NO_PAD.encode(self.secret),
            URL_SAFE_NO_PAD.encode(self.team)
        )
    }

    /// What the server receives of the link: the part before the `#`.
    pub fn server_part(url: &str) -> &str {
        url.split('#').next().unwrap_or(url)
    }

    pub fn parse(url: &str) -> Result<Self, Error> {
        let bad = || Error::Invitation(InvitationError::Malformed);
        let rest = url.strip_prefix("https://").ok_or_else(bad)?;
        let (before, fragment) = rest.split_once('#').ok_or_else(bad)?;
        let (server, id_hex) = before.split_once("/join/").ok_or_else(bad)?;
        let mut id = [0u8; 16];
        hex::decode_to_slice(id_hex, &mut id).map_err(|_| bad())?;
        let mut secret = None;
        let mut team = None;
        for pair in fragment.split('&') {
            match pair.split_once('=') {
                Some(("s", v)) => secret = Some(decode32(v).ok_or_else(bad)?),
                Some(("t", v)) => team = Some(decode32(v).ok_or_else(bad)?),
                _ => return Err(bad()),
            }
        }
        Ok(Self {
            server: server.to_string(),
            id,
            secret: secret.ok_or_else(bad)?,
            team: team.ok_or_else(bad)?,
        })
    }

    pub fn team(&self) -> TeamFingerprint {
        TeamFingerprint(self.team)
    }
}

fn decode32(text: &str) -> Option<[u8; 32]> {
    URL_SAFE_NO_PAD.decode(text).ok()?.try_into().ok()
}

/// What the invitee sends to the server, for an administrator to admit it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinRequest {
    #[serde(with = "hex::serde")]
    pub invitation: [u8; 16],
    /// The MLS KeyPackage, TLS-encoded (an `MlsMessageOut`).
    #[serde(with = "hex::serde")]
    pub key_package: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub mac: [u8; 32],
}

/// The Welcome, and the administrator's confirmation, for the invitee.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WelcomeDelivery {
    #[serde(with = "hex::serde")]
    pub welcome: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub confirmation: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvitationError {
    #[error("the invitation link is malformed")]
    Malformed,
    #[error("no such invitation")]
    Unknown,
    #[error("the invitation was already used")]
    AlreadyUsed,
    #[error("the invitation has expired")]
    Expired,
    #[error(
        "the request does not prove the invitation's secret: someone put another key in its place"
    )]
    BadMac,
    #[error("the group joined is not the team of the link")]
    WrongTeam,
    #[error("the Welcome does not come from the administrator who invited")]
    BadConfirmation,
}

fn hmac(secret: &[u8; 32], parts: &[&[u8]]) -> Hmac<Sha256> {
    let mut mac =
        <Hmac<Sha256> as KeyInit>::new_from_slice(secret).expect("HMAC takes any key length");
    for p in parts {
        mac.update(&(p.len() as u32).to_be_bytes());
        mac.update(p);
    }
    mac
}

pub fn join_mac(secret: &[u8; 32], invitation: &[u8; 16], key_package: &[u8]) -> [u8; 32] {
    hmac(secret, &[b"escouade/join/v1", invitation, key_package])
        .finalize()
        .into_bytes()
        .into()
}

pub fn verify_join_mac(
    secret: &[u8; 32],
    invitation: &[u8; 16],
    key_package: &[u8],
    tag: &[u8; 32],
) -> Result<(), InvitationError> {
    // verify_slice compares in constant time.
    hmac(secret, &[b"escouade/join/v1", invitation, key_package])
        .verify_slice(tag)
        .map_err(|_| InvitationError::BadMac)
}

pub fn welcome_confirmation(
    secret: &[u8; 32],
    group_id: &[u8],
    epoch: u64,
    epoch_authenticator: &[u8],
) -> [u8; 32] {
    hmac(
        secret,
        &[
            b"escouade/welcome/v1",
            group_id,
            &epoch.to_be_bytes(),
            epoch_authenticator,
        ],
    )
    .finalize()
    .into_bytes()
    .into()
}

pub fn verify_welcome_confirmation(
    secret: &[u8; 32],
    group_id: &[u8],
    epoch: u64,
    epoch_authenticator: &[u8],
    tag: &[u8; 32],
) -> Result<(), InvitationError> {
    hmac(
        secret,
        &[
            b"escouade/welcome/v1",
            group_id,
            &epoch.to_be_bytes(),
            epoch_authenticator,
        ],
    )
    .verify_slice(tag)
    .map_err(|_| InvitationError::BadConfirmation)
}
