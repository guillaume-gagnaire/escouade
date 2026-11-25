//! The vectors' model in the app (ESC-254): what the settings show of it, its download (its progress told to the
//! window, called off by « Annuler »), its deletion. One model for every project of this machine.

use super::model::{Spec, Store, MODEL, MODEL_URL};
use crate::core::Core;
use crate::voice::models::Cancelled;
use anyhow::{bail, Result};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Runtime;

/// The model, as « Recherche par le sens » shows it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VectorsModelInfo {
    pub id: String,
    pub license: String,
    /// Its size to download, in bytes.
    pub size: u64,
    /// What is on the disk toward it (a download cut keeps its part).
    pub have: u64,
    pub downloaded: bool,
    pub downloading: bool,
}

/// Where the model comes from: `ESCOUADE_VECTORS_MODEL_URL` (a fake server of the end-to-end tests), else Hugging Face.
fn base_url() -> String {
    std::env::var("ESCOUADE_VECTORS_MODEL_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| MODEL_URL.to_string())
}

impl<R: Runtime> Core<R> {
    fn vectors_store(&self) -> Store {
        Store {
            dir: self.data.vectors_model(MODEL.id),
        }
    }

    pub fn vectors_model_info(&self) -> VectorsModelInfo {
        info(
            &self.vectors_store(),
            &MODEL,
            self.vectors_download.lock().is_some(),
        )
    }

    /// Downloads the model, its progress told (`UiEvent::VectorsDownload`), through the proxy of the settings. Called
    /// off by `vectors_model_cancel`: what came is kept, the next download goes on from there.
    pub async fn vectors_model_download(&self) -> Result<VectorsModelInfo> {
        let store = self.vectors_store();
        if store.has(&MODEL) {
            return Ok(self.vectors_model_info());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut under_way = self.vectors_download.lock();
            if under_way.is_some() {
                bail!(tr!(
                    "Le modèle est déjà en cours de téléchargement.",
                    "The model is already being downloaded."
                ));
            }
            *under_way = Some(cancel.clone());
        }
        let client = crate::voice::models::client(&self.settings.read());
        let r = match client {
            Ok(client) => {
                let hub = &self.hub;
                store
                    .download(&client, &base_url(), &MODEL, &cancel, |done, total| {
                        hub.emit(crate::model::UiEvent::VectorsDownload { done, total })
                    })
                    .await
            }
            Err(e) => Err(e),
        };
        *self.vectors_download.lock() = None;
        match r {
            Ok(()) => Ok(self.vectors_model_info()),
            // « Annuler »: not an error to tell.
            Err(e) if e.is::<Cancelled>() => Ok(self.vectors_model_info()),
            Err(e) => Err(crate::voice::models::download_error(
                e,
                crate::voice::models::Downloaded::Model,
            )),
        }
    }

    /// « Annuler »: the download under way stops at its next piece.
    pub fn vectors_model_cancel(&self) {
        if let Some(flag) = self.vectors_download.lock().as_ref() {
            flag.store(true, Ordering::Release);
        }
    }

    /// « Effacer le modèle »: its files go, and what a download left; the search goes back to the words alone.
    pub fn vectors_model_delete(&self) -> Result<VectorsModelInfo> {
        self.vectors_model_cancel();
        self.vectors_store().delete(&MODEL)?;
        Ok(self.vectors_model_info())
    }
}

fn info(store: &Store, spec: &Spec, downloading: bool) -> VectorsModelInfo {
    VectorsModelInfo {
        id: spec.id.to_string(),
        license: spec.license.to_string(),
        size: spec.size(),
        have: store.have(spec),
        downloaded: store.has(spec),
        downloading,
    }
}
