//! The models and the voices of each service on line (ESC-132), as the app keeps them: what its plugin read from the API,
//! kept in the app's data folder (`online-catalog.json`). The lists themselves are the plugin's (`escouade_voice::catalog`).

use super::online::Provider;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use escouade_voice::catalog::{
    catalog, default_model, shipped_models, Listed, Purpose, ServiceCatalog,
};

/// Where what was listed is kept, by service.
#[derive(Debug, Clone)]
pub struct CatalogStore {
    pub path: PathBuf,
}

impl CatalogStore {
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            path: dir.join("online-catalog.json"),
        }
    }

    pub fn all(&self) -> BTreeMap<String, Listed> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn get(&self, p: Provider) -> Option<Listed> {
        self.all().remove(p.id())
    }

    pub fn put(&self, p: Provider, listed: &Listed) -> anyhow::Result<()> {
        let mut all = self.all();
        all.insert(p.id().into(), listed.clone());
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Ok(crate::paths::write_atomic(
            &self.path,
            serde_json::to_string_pretty(&all)?.as_bytes(),
        )?)
    }

    /// `chosen` if `p` still offers it for `purpose` (or nothing was listed to tell), else its default; and, then,
    /// what to say of it.
    pub fn checked_model(
        &self,
        p: Provider,
        purpose: Purpose,
        chosen: &str,
    ) -> (String, Option<String>) {
        let default = default_model(p, purpose);
        let chosen = chosen.trim();
        if chosen.is_empty() || chosen == default {
            return (default.into(), None);
        }
        let listed = self.get(p);
        let offered = match purpose {
            Purpose::Transcribe => listed.and_then(|l| l.transcribe),
            Purpose::Speak => listed.and_then(|l| l.speak),
        }
        .unwrap_or_else(|| shipped_models(p, purpose));
        if offered.iter().any(|m| m.id == chosen) {
            return (chosen.into(), None);
        }
        let label = p.label();
        log::info!("voice: {label} no longer offers {chosen}: {default} taken");
        (
            default.into(),
            Some(tr!(
                "{label} ne propose plus le modèle « {chosen} » : « {default} », son modèle par défaut, le remplace.",
                "{label} no longer offers the model “{chosen}”: “{default}”, its default model, stands in."
            )),
        )
    }
}

#[cfg(test)]
#[path = "online_catalog_tests.rs"]
mod tests;
