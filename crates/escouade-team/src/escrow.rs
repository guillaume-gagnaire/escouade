//! The organisation's escrow key (C7-B), as text: what an administrator pastes when creating the team (its public
//! half and who holds it), and what the organisation keeps (its secret, the seed of the pair). Every epoch's record
//! is sealed to it, so the organisation reads all the content; every member is told so.
//!
//! - public: `escouade-escrow-public:1:<HPKE public key, base64url>`;
//! - secret: `escouade-escrow-secret:1:<seed, base64url>`, which gives the pair back ([`EscrowKey::from_seed`]).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use escouade_team_crypto::roster::EscrowPublic;
use escouade_team_crypto::EscrowKey;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider;
use zeroize::Zeroizing;

use crate::Error;

pub const PUBLIC_PREFIX: &str = "escouade-escrow-public:1:";
pub const SECRET_PREFIX: &str = "escouade-escrow-secret:1:";

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

/// A new key pair for the organisation: its public half for the team, and its secret, to give to whoever holds it.
pub fn new_pair(holder: &str) -> Result<(EscrowPublic, Zeroizing<String>), Error> {
    let holder = holder_of(holder)?;
    let provider = OpenMlsRustCrypto::default();
    let key = EscrowKey::generate(provider.crypto(), provider.rand(), &holder)?;
    let secret = Zeroizing::new(format!(
        "{SECRET_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(key.seed())
    ));
    Ok((key.public(), secret))
}

pub fn public_text(public: &EscrowPublic) -> String {
    format!(
        "{PUBLIC_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(&public.hpke_key)
    )
}

/// The public half pasted, and who holds it.
pub fn parse_public(text: &str, holder: &str) -> Result<EscrowPublic, Error> {
    let bad = || Error::Refused("this is not the public half of an escrow key".into());
    let key = text.trim().strip_prefix(PUBLIC_PREFIX).ok_or_else(bad)?;
    let key = URL_SAFE_NO_PAD.decode(key).map_err(|_| bad())?;
    if key.len() != 32 {
        return Err(bad());
    }
    Ok(EscrowPublic {
        hpke_key: key,
        holder: holder_of(holder)?,
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
        let (public, secret) = new_pair("  le RSSI d'ACME ").unwrap();
        assert_eq!(public.holder, "le RSSI d'ACME");
        let text = public_text(&public);
        assert!(text.starts_with(PUBLIC_PREFIX));
        assert_eq!(parse_public(&text, "le RSSI d'ACME").unwrap(), public);
        let again = open_secret(&secret, "le RSSI d'ACME").unwrap();
        assert_eq!(again.public(), public);
        assert!(parse_public(&secret, "x").is_err());
        assert!(parse_public(&text, " ").is_err());
        assert!(open_secret(&text, "x").is_err());
    }
}
