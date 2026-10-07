//! The app's own updates (GitHub Releases), checked and installed here rather than by the
//! window through the updater plugin: they then follow the network settings as they are now (the
//! proxy, TLS verification off), which the plugin only takes from its build-time configuration.

use crate::model::Settings;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

/// How long a check may take (a download has no limit: a large installer behind a slow proxy
/// takes its time).
const TIMEOUT: Duration = Duration::from_secs(20);

/// An update found, as the window is told of it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// What installs or frees it.
    pub id: u32,
    pub version: String,
    pub notes: String,
}

/// What was found and not freed yet, by id: each check holds the proxy and the signature it was
/// made with, for its install.
pub struct Kept<T> {
    next: AtomicU32,
    items: Mutex<HashMap<u32, T>>,
}

impl<T> Default for Kept<T> {
    fn default() -> Self {
        Self {
            next: AtomicU32::new(1),
            items: Mutex::default(),
        }
    }
}

impl<T: Clone> Kept<T> {
    pub fn keep(&self, item: T) -> u32 {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        self.items.lock().insert(id, item);
        id
    }

    pub fn get(&self, id: u32) -> Option<T> {
        self.items.lock().get(&id).cloned()
    }

    pub fn free(&self, id: u32) {
        self.items.lock().remove(&id);
    }
}

pub type Updates = Kept<Update>;

/// Looks for a newer release through the proxy, its certificate checked unless TLS verification
/// is off; the update found is kept for its install.
pub async fn check<R: Runtime>(
    app: &AppHandle<R>,
    settings: &Settings,
    kept: &Updates,
) -> Result<Option<Found>> {
    let mut builder = app.updater_builder().timeout(TIMEOUT);
    let proxy = settings.proxy_url.trim();
    if !proxy.is_empty() {
        builder = builder.proxy(
            proxy
                .parse()
                .map_err(|e| anyhow!("adresse du proxy invalide : {e}"))?,
        );
    }
    if settings.insecure_tls {
        builder = builder.configure_client(|c| c.danger_accept_invalid_certs(true));
    }
    let Some(update) = builder.build()?.check().await? else {
        return Ok(None);
    };
    let (version, notes) = (
        update.version.clone(),
        update.body.clone().unwrap_or_default(),
    );
    Ok(Some(Found {
        id: kept.keep(update),
        version,
        notes,
    }))
}

/// Downloads the update `id` found, checks its signature and installs it (on Windows, the
/// installer then closes the app).
pub async fn install(kept: &Updates, id: u32) -> Result<()> {
    let update = kept.get(id).ok_or_else(|| {
        anyhow!("Cette mise à jour n'est plus proposée : recherche-la de nouveau.")
    })?;
    update.download_and_install(|_, _| {}, || {}).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_found_is_kept_until_freed_each_under_its_own_id() {
        let kept = Kept::<String>::default();
        let a = kept.keep("0.2.0".into());
        let b = kept.keep("0.2.0".into());
        assert_ne!(a, b);
        assert_eq!(kept.get(a).as_deref(), Some("0.2.0"));
        kept.free(a);
        assert_eq!(kept.get(a), None);
        assert_eq!(kept.get(b).as_deref(), Some("0.2.0"));
        // Freed twice, or never kept: nothing happens.
        kept.free(a);
        kept.free(99);
    }
}
