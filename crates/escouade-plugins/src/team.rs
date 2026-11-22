//! A version of a team's private store, as the app takes it (TS-14, `docs/TEAM-SERVER.md` §7): what the team's client
//! (`escouade_team::plugins`) read and opened, checked again here before anything is written, whatever the client
//! said. The hard refusals of the app live here:
//!
//! - the team's signature (its key declared to the team, minisign's prehashed form, the comment
//!   `escouade-team-v1 <id>@<version> <namespace>`) stands only for an id of the team's namespace;
//! - **never `dev.escouade.*`**, whatever the namespace says;
//! - never a namespace that holds a plugin installed here from another origin ([`super::Plugins::team_conflict`]);
//! - a plugin of the team never takes the place of a plugin of the same id from elsewhere (the store's, a folder's):
//!   a plugin's identity is its origin and its id together.
//!
//! The archive and the module are those the statement names, by their SHA-256.

use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;

/// What the trusted comment of a team's signature starts with.
pub const STATEMENT_COMMENT: &str = "escouade-team-v1";
/// Escouade's own ids: never a team's.
pub const FIRST_PARTY: &str = "dev.escouade";

/// A version of a team's store, opened by the team's client.
pub struct TeamFetched {
    /// The team's id on this device, and its name (« Acme · privé »).
    pub team: String,
    pub name: String,
    /// The team's fingerprint, as its statements name it.
    pub mark: String,
    /// The namespace's prefix the team declared (`fr.acme`), and its trusted keys.
    pub prefix: String,
    pub keys: Vec<String>,
    /// The statement the team's key signed, and its signature.
    pub statement: Vec<u8>,
    pub minisig: String,
    /// The archive (tar.gz) and the module the statement names.
    pub archive: Vec<u8>,
    pub module: Option<Vec<u8>>,
}

/// What the app reads of a statement.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statement {
    pub format: u32,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub tag: String,
    pub namespace: String,
    pub team: String,
    pub archive_sha256: String,
    pub module_sha256: Option<String>,
}

/// Whether the id `id` is under the prefix `prefix` (`fr.acme.rules` under `fr.acme`).
pub fn within(id: &str, prefix: &str) -> bool {
    let id = id.trim().to_ascii_lowercase();
    let prefix = prefix.trim().to_ascii_lowercase();
    !prefix.is_empty()
        && (id == prefix
            || id
                .strip_prefix(prefix.as_str())
                .is_some_and(|rest| rest.starts_with('.')))
}

/// Whether a team whose namespace is `prefix` may sign the plugin `id`: inside its namespace, never Escouade's own,
/// and never a namespace that holds Escouade's own (`dev`).
pub fn may_sign(prefix: &str, id: &str) -> bool {
    prefix.contains('.')
        && within(id, prefix)
        && !within(id, FIRST_PARTY)
        && !within(prefix, FIRST_PARTY)
        && !within(FIRST_PARTY, prefix)
}

/// The statement of `f`, when it holds: signed by one of the team's keys for this id, version and namespace, of this
/// team, inside its namespace, and the archive and the module the bytes it names. Why not, in words.
pub fn checked(f: &TeamFetched) -> Result<Statement, String> {
    let s: Statement = serde_json::from_slice(&f.statement)
        .map_err(|e| format!("the team's statement does not read: {e}"))?;
    if within(&s.id, FIRST_PARTY) {
        return Err(format!(
            "{} is a plugin of Escouade's own: a team never signs it",
            s.id
        ));
    }
    if !may_sign(&f.prefix, &s.id) {
        return Err(format!(
            "{} is not in the team's namespace {}.*",
            s.id, f.prefix
        ));
    }
    if s.format != 1 || s.namespace != f.prefix || s.team != f.mark {
        return Err("the statement is another team's, or of another namespace".into());
    }
    let sig = Signature::decode(&f.minisig).map_err(|_| "the team's signature does not read")?;
    let signed = f.keys.iter().any(|k| {
        PublicKey::from_base64(k.trim())
            .is_ok_and(|pk| pk.verify(&f.statement, &sig, false).is_ok())
    });
    if !signed {
        return Err("the team's key did not sign this version".into());
    }
    let comment = format!("{STATEMENT_COMMENT} {}@{} {}", s.id, s.version, s.namespace);
    if sig.trusted_comment() != comment {
        return Err("the team's signature is for another version".into());
    }
    if crate::store::sha256(&f.archive) != s.archive_sha256 {
        return Err("the archive is not the one the team signed".into());
    }
    if f.module.as_deref().map(crate::store::sha256) != s.module_sha256 {
        return Err("the module is not the one the team signed".into());
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_team_signs_its_namespace_alone() {
        assert!(may_sign("fr.acme", "fr.acme.rules"));
        assert!(!may_sign("fr.acme", "fr.acmeco.rules"));
        assert!(!may_sign("fr.acme", "com.other.x"));
        assert!(!may_sign("dev", "dev.escouade.github"));
        assert!(!may_sign("dev.escouade", "dev.escouade.github"));
        assert!(!may_sign("fr", "fr.acme.rules"), "a namespace of one label");
    }
}
