//! The dictation by a service on line, as the app sees it (ESC-110): which service the settings choose, and its key, kept
//! in the system's keychain (`secrets`), never in a file nor in the log. What goes to the service and what its answers mean
//! is its plugin's (`escouade_voice`, docs/PLUGINS.md §4.13): each call carries the key to it, and it keeps nothing.

use super::transcribe::Transcriber;
use crate::integrations::secrets::SecretStore;
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub use escouade_voice::online::{redact, OnlineError, Provider, Spec, PROVIDERS};

/// The service of this id: one of the five, or a plugin's registered one.
pub fn find(id: &str) -> Option<Provider> {
    Provider::core(id).or_else(|| super::service::find(id).map(|_| Provider::of(id)))
}

/// The specification of `p` with `key`: its address and the model it transcribes with when the user chose none.
fn spec_for(p: Provider, key: String) -> Spec {
    Spec {
        model: super::service::of(p).default_model(),
        ..Spec::new(p, key)
    }
}

/// The service of the settings, with its key from the keychain and the model chosen for it (its default when the
/// service no longer offers it); none when the dictation is local. No key kept: `NoKey`,
/// told before anything is listened to.
pub fn spec_of(
    settings: &crate::model::Settings,
    secrets: &dyn SecretStore,
    catalog: &super::online_catalog::CatalogStore,
) -> std::result::Result<Option<Spec>, super::VoiceError> {
    let Some(p) = super::online_provider(&settings.voice) else {
        return Ok(None);
    };
    let chosen = settings
        .voice
        .online_models
        .get(p.id())
        .map(String::as_str)
        .unwrap_or("");
    let (model, _) = catalog.checked_model(p, super::online_catalog::Purpose::Transcribe, chosen);
    // « Transcription en temps réel », on unless unticked (ESC-136).
    let live = settings
        .voice
        .online_live
        .get(p.id())
        .copied()
        .unwrap_or(true);
    let service = super::service::of(p);
    let live = live.then(|| service.live_model(&model));
    Ok(Some(
        keyed(p, settings, secrets)?
            .with_model(&model)
            .with_live(live.filter(|_| service.offers_live())),
    ))
}

/// The entry of the voice of the plugin Mistral, the former name of the Mistral account (ESC-327).
const MISTRAL_ENTRY: &str = "dictation-mistral";

/// The key under the entry `entry` of a service, for the plugin Mistral under the former name read first and copied
/// under the account's (the migration of INT-6).
fn read_key(secrets: &dyn SecretStore, entry: &str) -> anyhow::Result<Option<String>> {
    if entry == MISTRAL_ENTRY {
        let name = crate::plugins::accounts::mistral_vault_name();
        crate::integrations::secrets::read_entry(secrets, Some(entry), &name, entry)
    } else {
        secrets.get(entry)
    }
}

/// The key of the Mistral account, if the keychain gives one.
pub fn mistral_key(secrets: &dyn SecretStore) -> Option<String> {
    read_key(secrets, MISTRAL_ENTRY)
        .unwrap_or_else(|e| {
            log::warn!("voice: the system keychain did not give the key of Mistral: {e:#}");
            None
        })
        .filter(|k| !k.trim().is_empty())
}

/// What `p`'s key is when it has none of its own: for Mistral, the first Vibe account's (ESC-327).
fn borrowed_key(p: Provider, settings: &crate::model::Settings) -> Option<String> {
    (p == Provider::Mistral)
        .then(|| crate::accounts::vibe_key_for_voice(settings))
        .flatten()
        .map(|(_, key)| key)
}

/// `p` with its key from the keychain (the one of its dictation, ESC-132: one entry by service).
pub fn keyed(
    p: Provider,
    settings: &crate::model::Settings,
    secrets: &dyn SecretStore,
) -> std::result::Result<Spec, super::VoiceError> {
    let key = read_key(secrets, &p.entry())
        .unwrap_or_else(|e| {
            log::warn!(
                "voice: the system keychain did not give the key of {}: {e:#}",
                p.label()
            );
            None
        })
        .filter(|k| !k.trim().is_empty())
        .or_else(|| borrowed_key(p, settings))
        .ok_or(super::VoiceError::Online(OnlineError::NoKey(p)))?;
    Ok(spec_for(p, key))
}

/// The services whose key is in the keychain (never the keys).
pub fn keys(secrets: &dyn SecretStore) -> Vec<String> {
    super::service::services()
        .into_iter()
        .filter(|s| {
            read_key(secrets, &s.account_entry())
                .ok()
                .flatten()
                .is_some_and(|k| !k.trim().is_empty())
        })
        .map(|s| s.id())
        .collect()
}

/// `keys`, with the service whose key is another account's: Mistral on a Vibe account's (ESC-327).
pub fn usable_keys(settings: &crate::model::Settings, secrets: &dyn SecretStore) -> Vec<String> {
    let mut list = keys(secrets);
    let id = Provider::Mistral.id().to_string();
    if !list.contains(&id)
        && super::service::find(&id).is_some()
        && borrowed_key(Provider::Mistral, settings).is_some()
    {
        list.push(id);
    }
    list
}

/// Réglages › Voix and the accounts of Mistral Vibe: the key of the Mistral account, by its end, and the Vibe account
/// whose key the voice goes by when it has none.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MistralKey {
    pub own: Option<String>,
    pub borrowed: Option<Borrowed>,
}

/// A Vibe account the voice borrows its key from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Borrowed {
    pub account: String,
    pub hint: String,
}

/// What `MistralKey` says now.
pub fn mistral_state(settings: &crate::model::Settings) -> MistralKey {
    let own = crate::accounts::mistral_key_hint();
    let borrowed = crate::accounts::vibe_key_for_voice(settings)
        .filter(|_| own.is_none())
        .map(|(account, key)| Borrowed {
            account,
            hint: crate::accounts::key_hint(&key),
        });
    MistralKey { own, borrowed }
}

/// Keeps the key of `service` in the keychain, read back to check; never in a file: a keychain
/// that refuses it (or none, on Linux) leaves it unsaved, and says so, the key taken out.
pub fn save_key(
    secrets: &dyn SecretStore,
    service: &str,
    key: &str,
) -> std::result::Result<(), String> {
    let p = super::service::find(service).ok_or_else(|| format!("unknown service {service}"))?;
    let key = key.trim();
    if key.is_empty() {
        return Err(tr!("La clé est vide.", "The key is empty."));
    }
    keep_key(secrets, &p.account_entry(), key).map_err(|e| {
        let e = redact(&format!("{e:#}"), key);
        log::warn!(
            "voice: the system keychain refused the key of {}: {e}",
            p.label()
        );
        tr!(
            "Le trousseau du système refuse la clé ({e}) : elle n’est pas gardée, et jamais dans un fichier.",
            "The system keychain refuses the key ({e}): it is not kept, and never in a file."
        )
    })?;
    log::info!("voice: key of {} kept in the system keychain", p.label());
    Ok(())
}

/// The key kept under `entry` and, for the Mistral account, under its name of the plugin too.
fn keep_key(secrets: &dyn SecretStore, entry: &str, key: &str) -> anyhow::Result<()> {
    use crate::integrations::secrets::keep_secret;
    keep_secret(secrets, entry, key)?;
    if entry == MISTRAL_ENTRY {
        keep_secret(
            secrets,
            &crate::plugins::accounts::mistral_vault_name(),
            key,
        )?;
        crate::accounts::remember_mistral_key(Some(key));
    }
    Ok(())
}

/// Forgets the key of `service`, under each of its names.
pub fn delete_key(secrets: &dyn SecretStore, service: &str) -> std::result::Result<(), String> {
    let p = super::service::find(service).ok_or_else(|| format!("unknown service {service}"))?;
    let entry = p.account_entry();
    secrets.delete(&entry).map_err(|e| format!("{e:#}"))?;
    if entry == MISTRAL_ENTRY {
        secrets
            .delete(&crate::plugins::accounts::mistral_vault_name())
            .map_err(|e| format!("{e:#}"))?;
        crate::accounts::remember_mistral_key(None);
    }
    Ok(())
}

/// The transcriber of a dictation on line: what `Voice` calls, as it calls Whisper. It asks the service of the
/// contract (`service::VoiceService`), which runs the request on a runtime of its own, on a thread of its own:
/// `Voice::stop` blocks.
pub struct Online(pub Spec);

impl Transcriber for Online {
    fn transcribe(
        &self,
        _model: &Path,
        samples: &[f32],
        language: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String> {
        super::service::of(self.0.provider)
            .transcribe(&self.0, samples, language, cancel)
            .map_err(anyhow::Error::new)
    }
}

#[cfg(test)]
#[path = "online_tests.rs"]
pub(crate) mod tests;
