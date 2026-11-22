//! The organisation's escrow key (C7-B), as text: what an administrator pastes when creating the team (the team's
//! half, and who holds the key), and what the organisation keeps (its secret, the seed of the pair). Every epoch's
//! record is sealed to it, so the organisation reads all the content; every member is told so.
//!
//! - the team's half: `escouade-escrow-team:2:<HPKE public key, base64url>.<anchor's seed, base64url>`. Beside the
//!   public key, the anchor the administrators' devices sign the escrow wraps with (SR2-09, `escouade_team_crypto::
//!   anchor`): it reads nothing, but whoever holds it could sign a wrap, so it is pasted in Escouade only;
//! - secret: `escouade-escrow-secret:1:<seed, base64url>`, which gives the pair back ([`EscrowKey::from_seed`]), and
//!   the anchor with it: the organisation checks each wrap by it.
//!
//! The public half alone (`escouade-escrow-public:1:`, before the anchors) is refused: make a new pair.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use escouade_team_crypto::{Anchor, EscrowKey, EscrowTeamHalf};
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider;
use zeroize::Zeroizing;

use crate::Error;

pub const TEAM_PREFIX: &str = "escouade-escrow-team:2:";
pub const SECRET_PREFIX: &str = "escouade-escrow-secret:1:";
/// The text of a pair made before the anchors (SR2-09): refused, with what to do.
pub const OLD_PUBLIC_PREFIX: &str = "escouade-escrow-public:1:";

/// The longest name of a holder.
pub const MAX_HOLDER: usize = 120;

fn holder_of(holder: &str) -> Result<String, Error> {
    let holder = holder.trim();
    if holder.is_empty() || holder.chars().count() > MAX_HOLDER {
        return Err(Error::Refused(
            "an escrow key says who holds it, in 120 characters at most".into(),
        ));
    }
    Ok(holder.to_string())
}

/// A new key pair for the organisation: the team's half, and its secret, to give to whoever holds it.
pub fn new_pair(holder: &str) -> Result<(EscrowTeamHalf, Zeroizing<String>), Error> {
    let holder = holder_of(holder)?;
    let provider = OpenMlsRustCrypto::default();
    let key = EscrowKey::generate(provider.crypto(), provider.rand(), &holder)?;
    let secret = Zeroizing::new(format!(
        "{SECRET_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(key.seed())
    ));
    Ok((key.team_half(), secret))
}

/// The team's half as text, to paste when creating the team.
pub fn team_text(half: &EscrowTeamHalf) -> Zeroizing<String> {
    Zeroizing::new(format!(
        "{TEAM_PREFIX}{}.{}",
        URL_SAFE_NO_PAD.encode(&half.public.hpke_key),
        URL_SAFE_NO_PAD.encode(half.anchor.seed())
    ))
}

/// The team's half pasted, and who holds the key.
pub fn parse_team(text: &str, holder: &str) -> Result<EscrowTeamHalf, Error> {
    let text = text.trim();
    if text.starts_with(OLD_PUBLIC_PREFIX) {
        return Err(Error::Refused(
            "this escrow key was made before its wraps were signed: make a new pair".into(),
        ));
    }
    let bad = || Error::Refused("this is not the team's half of an escrow key".into());
    let rest = text.strip_prefix(TEAM_PREFIX).ok_or_else(bad)?;
    let (key, anchor) = rest.split_once('.').ok_or_else(bad)?;
    let key = URL_SAFE_NO_PAD.decode(key).map_err(|_| bad())?;
    let seed = Zeroizing::new(URL_SAFE_NO_PAD.decode(anchor).map_err(|_| bad())?);
    if key.len() != 32 {
        return Err(bad());
    }
    let seed: [u8; 32] = seed.as_slice().try_into().map_err(|_| bad())?;
    let anchor = Anchor::from_seed(seed);
    Ok(EscrowTeamHalf {
        public: escouade_team_crypto::roster::EscrowPublic {
            hpke_key: key,
            holder: holder_of(holder)?,
            anchor_key: anchor.public(),
        },
        anchor,
    })
}

/// The pair again from its secret (what the organisation kept).
pub fn open_secret(text: &str, holder: &str) -> Result<EscrowKey, Error> {
    let bad = || Error::Refused("this is not the secret of an escrow key".into());
    let seed = text.trim().strip_prefix(SECRET_PREFIX).ok_or_else(bad)?;
    let seed = Zeroizing::new(URL_SAFE_NO_PAD.decode(seed).map_err(|_| bad())?);
    let seed: [u8; 32] = seed.as_slice().try_into().map_err(|_| bad())?;
    let provider = OpenMlsRustCrypto::default();
    Ok(EscrowKey::from_seed(
        provider.crypto(),
        seed,
        &holder_of(holder)?,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pair_goes_to_text_and_back() {
        let (half, secret) = new_pair("  le RSSI d'ACME ").unwrap();
        assert_eq!(half.public.holder, "le RSSI d'ACME");
        let text = team_text(&half);
        assert!(text.starts_with(TEAM_PREFIX));
        let back = parse_team(&text, "le RSSI d'ACME").unwrap();
        assert_eq!(back.public, half.public);
        assert_eq!(back.anchor.public(), half.anchor.public());
        let again = open_secret(&secret, "le RSSI d'ACME").unwrap();
        // The organisation's seed gives the same public half and the same anchor.
        assert_eq!(again.public(), half.public);
        assert_eq!(again.anchor().public(), half.anchor.public());
        assert!(parse_team(&secret, "x").is_err());
        assert!(parse_team(&text, " ").is_err());
        assert!(open_secret(&text, "x").is_err());
    }

    #[test]
    fn a_public_half_without_its_anchor_is_refused() {
        let old = format!("{OLD_PUBLIC_PREFIX}{}", URL_SAFE_NO_PAD.encode([1u8; 32]));
        let refused = parse_team(&old, "Org").unwrap_err().to_string();
        assert!(refused.contains("make a new pair"), "{refused}");
        let cut = format!("{TEAM_PREFIX}{}", URL_SAFE_NO_PAD.encode([1u8; 32]));
        assert!(parse_team(&cut, "Org").is_err());
    }
}
