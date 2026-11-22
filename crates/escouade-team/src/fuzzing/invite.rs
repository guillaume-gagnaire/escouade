//! The invitation (C6-A, TS-3): what an invitee pastes, what it sends, what it receives.
//! - The bytes as a link: one that parses gives itself back (`to_url`), names a host without a path, a user or a
//!   space, and the part the server receives holds neither its secret nor its team.
//! - A request to join made with a known secret, changed by the bytes: its MAC holds for the very invitation and
//!   KeyPackage it was made for, never for another (the server cannot put its own key in place).
//! - A Welcome's confirmation, changed: it holds for the group, epoch and authenticator it was made for alone.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use escouade_team_crypto::invite::{
    verify_join_mac, verify_welcome_confirmation, welcome_confirmation, JoinRequest,
    WelcomeDelivery,
};
use escouade_team_crypto::{InviteLink, TeamDevice};

use super::{alter, member};
use crate::wire;

const SECRET: [u8; 32] = [5; 32];

fn request() -> &'static JoinRequest {
    static REQUEST: std::sync::OnceLock<JoinRequest> = std::sync::OnceLock::new();
    REQUEST.get_or_init(|| {
        let link = InviteLink {
            server: "team.example.org".into(),
            id: [4; 16],
            secret: SECRET,
            team: [6; 32],
            member: None,
        };
        let mut device = TeamDevice::new(member(9)).expect("a device");
        device.request_join(&link).expect("a request")
    })
}

pub fn invite(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(link) = InviteLink::parse(text) {
            let url = link.to_url();
            assert_eq!(InviteLink::parse(&url).expect("again"), link);
            assert!(
                !link.server.is_empty() && !link.server.contains(['/', '?', '@', ' ', '#']),
                "a host that is not one: {}",
                link.server
            );
            // The link opens the app, never a web page whose script would read its secret.
            assert!(url.starts_with("escouade://team/join/"), "{url}");
            assert!(url.contains(&URL_SAFE_NO_PAD.encode(link.secret)));
            let _ = link.server_token();
        }
    }
    let genuine = request();
    assert!(verify_join_mac(
        &SECRET,
        &genuine.invitation,
        &genuine.key_package,
        &genuine.mac
    )
    .is_ok());
    if let Ok(r) = wire::decode::<JoinRequest>(&alter(&wire::encode(genuine), data)) {
        if verify_join_mac(&SECRET, &r.invitation, &r.key_package, &r.mac).is_ok() {
            assert_eq!(
                (&r.invitation, &r.key_package),
                (&genuine.invitation, &genuine.key_package),
                "a request changed and its MAC still holds"
            );
        }
    }
    if let Ok(r) = wire::decode::<JoinRequest>(data) {
        let _ = verify_join_mac(&SECRET, &r.invitation, &r.key_package, &r.mac);
    }
    // A Welcome's confirmation, for a group, an epoch and an authenticator the bytes change.
    let (group, epoch, auth) = (b"the group".to_vec(), 3u64, [8u8; 32]);
    let tag = welcome_confirmation(&SECRET, &group, epoch, &auth);
    let mut cited = (group.clone(), epoch, auth.to_vec(), tag);
    for (i, b) in data.iter().enumerate() {
        match i % 4 {
            0 => cited.0.push(*b),
            1 => cited.1 ^= u64::from(*b),
            2 => cited.2[usize::from(*b) % 32] ^= b.rotate_left(1) | 1,
            _ => cited.3[usize::from(*b) % 32] ^= b | 1,
        }
        if verify_welcome_confirmation(&SECRET, &cited.0, cited.1, &cited.2, &cited.3).is_ok() {
            assert_eq!(
                (&cited.0, cited.1, &cited.2[..]),
                (&group, epoch, &auth[..]),
                "a confirmation that holds for another group or epoch"
            );
        }
    }
    let _ = wire::decode::<WelcomeDelivery>(data);
}
