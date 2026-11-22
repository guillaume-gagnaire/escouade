//! The invitation (C6-A): the link carries an authentication secret only, after the `#`; the keys come from an
//! administrator online, by an MLS Welcome.
//!
//! 1. The administrator's device draws an id (16 bytes) and a secret `s` (32 bytes), and keeps the [`Invitation`]:
//!    role, expiry, single use.
//! 2. The link: `escouade://team/join/<server>/<id>#s=<s>&t=<team fingerprint>` (base64url), and `&m=<member>` for
//!    another device of a member already there (the QR code of §3.4). It opens Escouade (a click, or pasted in it),
//!    never a web page: a page the team server serves would read the secret after the `#` in the browser
//!    (`location.hash`), and a server taken over would enter in the invitee's place (SR2-08). The invitee's token
//!    for the server is derived from `s` ([`server_token`]): the administrator registers its SHA-256 with the server,
//!    which never sees `s` and lets the token serve once.
//! 3. The invitee's device makes its keys and an MLS KeyPackage, and sends a [`JoinRequest`]: the KeyPackage and
//!    `HMAC-SHA256(s, id ‖ KeyPackage)`. The server, without `s`, cannot put its own KeyPackage in its place.
//! 4. An administrator's device checks the MAC, commits the Add (and the roster's new member), and sends the
//!    Welcome with a confirmation `HMAC-SHA256(s, group ‖ epoch ‖ epoch authenticator)`.
//! 5. The invitee checks that the group it joined has the team fingerprint of its link and the confirmation: the
//!    server cannot show it another team.
//!
//! The links of before (`https://<server>/join/<id>#…`, which opened the server's page) are refused: by the invitee's
//! app ([`InvitationError::OldForm`], to ask for a new one), and by the inviting device, which admits no one with an
//! invitation made before ([`Invitation::app_link`]).
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
    /// Its link is the app's (`escouade://`), never a page of the server (SR2-08). False for an invitation made
    /// before, whose `https://` link may have been opened on the server's page: it lets no one in.
    #[serde(default)]
    pub app_link: bool,
}

/// The link, parsed.
#[derive(Clone, Debug, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct InviteLink {
    pub server: String,
    pub id: [u8; 16],
    pub secret: [u8; 32],
    pub team: [u8; 32],
    /// The member whose other device this is (`Purpose::OwnDevice`); none for a new member, who draws their id.
    pub member: Option<[u8; 16]>,
}

/// The invitee's one-use token for the server, derived from the invitation's secret: the administrator registers its
/// SHA-256 (the server keeps nothing else), the invitee enrols with it. Knowing it does not give `s` back.
pub fn server_token(secret: &[u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(crate::keys::expand(
        secret,
        b"escouade/invite/server-token/v1",
        b"",
    ))
}

/// The scheme and path of a link: what the app registers (`escouade://`) and reads.
pub const LINK_PREFIX: &str = "escouade://team/join/";

impl InviteLink {
    /// `escouade://team/join/<server>/<id>#s=…&t=…` (and `&m=…`): opens Escouade, never a web page.
    pub fn to_url(&self) -> String {
        let mut url = format!(
            "{LINK_PREFIX}{}/{}#s={}&t={}",
            self.server,
            hex::encode(self.id),
            URL_SAFE_NO_PAD.encode(self.secret),
            URL_SAFE_NO_PAD.encode(self.team)
        );
        if let Some(m) = self.member {
            url.push_str(&format!("&m={}", hex::encode(m)));
        }
        url
    }

    /// The invitee's token for the server ([`server_token`]).
    pub fn server_token(&self) -> String {
        server_token(&self.secret)
    }

    /// Reads a link: `escouade://team/join/<server>/<id>#…` (the scheme and path in any case, a `/` after the id
    /// allowed). A link of before (`https://<server>/join/<id>#…`) is [`InvitationError::OldForm`]: it may have
    /// opened the server's page, which reads the secret.
    pub fn parse(url: &str) -> Result<Self, Error> {
        let bad = || Error::Invitation(InvitationError::Malformed);
        let head = url.get(..LINK_PREFIX.len()).ok_or_else(bad)?;
        if !head.eq_ignore_ascii_case(LINK_PREFIX) {
            // Of the old form, also as typed for a server on this machine (`http://`).
            let https = url.replacen("http://", "https://", 1);
            return Err(match Self::parse_old(&https) {
                Ok(_) => Error::Invitation(InvitationError::OldForm),
                Err(e) => e,
            });
        }
        let (before, fragment) = url[LINK_PREFIX.len()..].split_once('#').ok_or_else(bad)?;
        let before = before.strip_suffix('/').unwrap_or(before);
        let (server, id_hex) = before.split_once('/').ok_or_else(bad)?;
        Self::parts(server, id_hex, fragment)
    }

    /// A link as a device kept it while joining: of either form, for a device that began to join before the links
    /// changed. Never for a link given by someone ([`InviteLink::parse`]).
    pub fn parse_kept(url: &str) -> Result<Self, Error> {
        Self::parse(url).or_else(|_| Self::parse_old(url))
    }

    /// The form of before: `https://<server>/join/<id>#…`.
    fn parse_old(url: &str) -> Result<Self, Error> {
        let bad = || Error::Invitation(InvitationError::Malformed);
        let rest = url.strip_prefix("https://").ok_or_else(bad)?;
        let (before, fragment) = rest.split_once('#').ok_or_else(bad)?;
        let (server, id_hex) = before.split_once("/join/").ok_or_else(bad)?;
        Self::parts(server, id_hex, fragment)
    }

    fn parts(server: &str, id_hex: &str, fragment: &str) -> Result<Self, Error> {
        let bad = || Error::Invitation(InvitationError::Malformed);
        let mut id = [0u8; 16];
        hex::decode_to_slice(id_hex, &mut id).map_err(|_| bad())?;
        if server.is_empty() || server.contains(['/', '?', '@', ' ', '#', '\\']) {
            return Err(bad());
        }
        let mut secret = None;
        let mut team = None;
        let mut member = None;
        for pair in fragment.split('&') {
            match pair.split_once('=') {
                Some(("s", v)) if secret.is_none() => secret = Some(decode32(v).ok_or_else(bad)?),
                Some(("t", v)) if team.is_none() => team = Some(decode32(v).ok_or_else(bad)?),
                Some(("m", v)) if member.is_none() => {
                    let mut m = [0u8; 16];
                    hex::decode_to_slice(v, &mut m).map_err(|_| bad())?;
                    member = Some(m);
                }
                _ => return Err(bad()),
            }
        }
        Ok(Self {
            server: server.to_string(),
            id,
            secret: secret.ok_or_else(bad)?,
            team: team.ok_or_else(bad)?,
            member,
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
    #[error("the invitation link is of the old form (https://…/join/…): ask for a new one")]
    OldForm,
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
