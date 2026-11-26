//! An install, or an update (an install of a newer version): in this order, each step stopping the rest.
//!
//! 1. the allowed list, on the id alone: an extension refused is not even asked of Open VSX;
//! 2. its card (the version asked, or the latest), and the allowed list again on its version;
//! 3. the trust of its publisher (VS Code's question, asked by the page): a publisher not trusted yet stops here;
//! 4. its VSIX, its sum, its signature and Open VSX's key, each within a bound; a version with no signature stops here;
//! 5. the sum, then the signature, checked; the archive unpacked in a scratch folder, every entry checked first;
//! 6. its `package.json`, which must be the card's extension and version;
//! 7. put in the store, in place of the version before.

use crate::allowed::{self, Layer};
use crate::manifest::{self, Kind};
use crate::registry::{compare_versions, Registry};
use crate::store::{Installed, Store};
use crate::unpack::{self, Limits};
use crate::verify;
use crate::{Code, ExtId, Refusal, Result};
use serde::Serialize;

/// `id` (at `version`, its latest when none) installed, or updated. `trust`: the user answered yes to the question of
/// its publisher (kept then).
pub async fn install(
    registry: &Registry,
    store: &Store,
    limits: &Limits,
    id: &str,
    version: Option<&str>,
    layers: &[Layer],
    trust: bool,
) -> Result<Installed> {
    let id = ExtId::parse(id)?;
    allowed::check(layers, &id, None)?;
    let card = registry.detail(&id, version).await?;
    allowed::check(layers, &id, Some((&card.version, card.pre_release)))?;
    if !store.trusts(&card.namespace) {
        if !trust {
            let publisher = &card.publisher;
            return Err(Refusal::new(
                Code::Untrusted,
                tr!(
                    "Faites-vous confiance à l’éditeur « {publisher} » ?",
                    "Do you trust the publisher “{publisher}”?"
                ),
            ));
        }
        store.trust(&card.namespace)?;
    }
    let files = &card.files;
    let Some(download) = files.download.as_deref().filter(|_| card.downloadable) else {
        return Err(Refusal::new(
            Code::NotFound,
            tr!(
                "Open VSX ne donne pas d’archive pour cette version.",
                "Open VSX gives no archive for this version."
            ),
        ));
    };
    let (Some(sum), Some(signature), Some(key)) = (
        files.sha256.as_deref(),
        files.signature.as_deref(),
        files.public_key.as_deref(),
    ) else {
        let v = &card.version;
        return Err(Refusal::new(
            Code::Unsigned,
            tr!(
                "La version {v} n’est pas signée par Open VSX : Escouade ne l’installe pas.",
                "Version {v} is not signed by Open VSX: Escouade does not install it."
            ),
        ));
    };
    let vsix = registry.bytes(download, limits.vsix).await?;
    let sum = String::from_utf8_lossy(&registry.bytes(sum, 1024).await?).into_owned();
    let sha256 = verify::check_sum(&vsix, &sum)?;
    let sigzip = registry.bytes(signature, 64 << 10).await?;
    // Open VSX's key pinned (SR4-09): the one it serves is read from the fake registry of development alone.
    match registry.pinned_key() {
        Some(pinned) => verify::check_signature(&vsix, &sigzip, &pinned).map_err(|r| {
            if r.code == Code::BadSignature {
                verify::not_the_pinned_key()
            } else {
                r
            }
        })?,
        None => {
            let pem = String::from_utf8_lossy(&registry.bytes(key, 8 << 10).await?).into_owned();
            verify::check_signature(&vsix, &sigzip, &verify::public_key(&pem)?)?
        }
    }

    let scratch = store.scratch()?;
    let done = async {
        let unpacked = unpack::unpack(&vsix, &scratch, limits)?;
        let bytes = std::fs::read(scratch.join("package.json")).map_err(Refusal::io)?;
        let m = manifest::read(&bytes, &id, &card.version)?;
        let display = m
            .raw
            .get("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or(&card.display_name)
            .to_owned();
        let e = Installed {
            id: card.id.clone(),
            key: id.key(),
            namespace: card.namespace.clone(),
            name: card.name.clone(),
            version: card.version.clone(),
            display_name: display,
            description: card.description.clone(),
            publisher: card.publisher.clone(),
            kind: m.kind,
            enabled: true,
            verified: card.verified,
            pre_release: card.pre_release,
            sha256,
            signed: true,
            installed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            files: unpacked.files,
            bytes: unpacked.bytes,
            contributes: m.contributes,
            dir: String::new(),
        };
        store.put(e, &scratch)
    }
    .await;
    if done.is_err() {
        let _ = std::fs::remove_dir_all(&scratch);
    }
    done
}

/// A newer version of an installed extension: proposed, never installed by itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub id: String,
    pub from: String,
    pub to: String,
    /// Why it would be refused (the allowed list), if it would.
    pub refused: Option<String>,
}

/// The updates of what is installed (one card asked per extension). An extension Open VSX no longer answers for is
/// left out.
pub async fn updates(registry: &Registry, store: &Store, layers: &[Layer]) -> Vec<Update> {
    let mut out = Vec::new();
    for e in store.list() {
        let Ok(id) = ExtId::parse(&e.id) else {
            continue;
        };
        if allowed::check(layers, &id, None).is_err() {
            continue;
        }
        let Ok(card) = registry.detail(&id, None).await else {
            continue;
        };
        if compare_versions(&card.version, &e.version).is_gt() {
            out.push(Update {
                id: e.id.clone(),
                from: e.version.clone(),
                to: card.version.clone(),
                refused: allowed::check(layers, &id, Some((&card.version, card.pre_release)))
                    .err()
                    .map(|r| r.message),
            });
        }
    }
    out
}

/// What the view says of an installed extension, besides its card: whether the allowed list refuses it now (it is
/// then not loaded), and what it waits for when it has code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(flatten)]
    pub installed: Installed,
    pub refused: Option<String>,
    /// « En attente de l'hôte d'extensions », « Demande l'hôte Node ».
    pub waits: Option<String>,
}

pub fn state(e: Installed, layers: &[Layer]) -> State {
    let refused = ExtId::parse(&e.id)
        .and_then(|id| allowed::check(layers, &id, Some((&e.version, e.pre_release))))
        .err()
        .map(|r| r.message);
    let waits = match e.kind {
        Kind::Declarative => None,
        Kind::Web => Some(tr!(
            "En attente de l’hôte d’extensions : son code ne tourne pas encore dans Escouade.",
            "Waiting for the extension host: its code does not run in Escouade yet."
        )),
        Kind::Node => Some(tr!(
            "Demande l’hôte Node, pas encore dans Escouade.",
            "Needs the Node host, not in Escouade yet."
        )),
    };
    State {
        installed: e,
        refused,
        waits,
    }
}
