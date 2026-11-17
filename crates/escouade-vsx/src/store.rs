//! What is installed, in Escouade's data folder (`<data>/extensions`): one folder per extension and version
//! (`<publisher>.<name>-<version>`, its `extension/` part alone), and `installed.json`, the list of them, the publishers
//! the user trusted (VS Code's question) and the colour theme of an extension chosen in the view (none: Escouade's).
//! Only what passed the checks of `install` comes here: what is in the list is what `serve` serves.

use crate::manifest::{Contributes, Kind};
use crate::{Refusal, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// An extension installed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    /// `publisher.name`, as Open VSX writes it.
    pub id: String,
    /// The same in lower case: how it is found.
    pub key: String,
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    /// The publisher's name, as Open VSX shows it.
    pub publisher: String,
    pub kind: Kind,
    pub enabled: bool,
    /// Whether its namespace was verified on Open VSX when it was installed.
    pub verified: bool,
    pub pre_release: bool,
    /// The SHA-256 of its VSIX, checked, and the signature that was checked with it.
    pub sha256: String,
    pub signed: bool,
    /// Seconds since 1970.
    pub installed_at: u64,
    pub files: usize,
    pub bytes: u64,
    pub contributes: Contributes,
    /// Its folder, under the store's.
    pub dir: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Index {
    extensions: Vec<Installed>,
    /// The publishers trusted, in lower case.
    trusted: Vec<String>,
    /// The colour theme chosen in the view: `<extension key>/<theme label>`.
    theme: Option<String>,
}

pub struct Store {
    root: PathBuf,
    index: Mutex<Index>,
}

const INDEX: &str = "installed.json";
const TMP: &str = ".tmp";

impl Store {
    /// The store of `root`, read; what a folder no longer holds leaves the list, and what an install left half-made
    /// goes.
    pub fn open(root: &Path) -> Store {
        let mut index: Index = std::fs::read(root.join(INDEX))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        index.extensions.retain(|e| {
            crate::serve::is_dir_name(&e.dir) && root.join(&e.dir).join("package.json").is_file()
        });
        let _ = std::fs::remove_dir_all(root.join(TMP));
        Store {
            root: root.to_owned(),
            index: Mutex::new(index),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn save(&self, index: &Index) -> Result<()> {
        std::fs::create_dir_all(&self.root).map_err(Refusal::io)?;
        let bytes = serde_json::to_vec_pretty(index).map_err(Refusal::io)?;
        escouade_plugin_cli::paths::write_atomic(&self.root.join(INDEX), &bytes)
            .map_err(Refusal::io)
    }

    pub fn list(&self) -> Vec<Installed> {
        self.index.lock().extensions.clone()
    }

    /// The extension of `key` (`publisher.name`, any case).
    pub fn get(&self, key: &str) -> Option<Installed> {
        let key = key.to_lowercase();
        self.index
            .lock()
            .extensions
            .iter()
            .find(|e| e.key == key)
            .cloned()
    }

    /// A new, empty folder to unpack into, under the store (on the same disk, so that it is moved, not copied).
    pub fn scratch(&self) -> Result<PathBuf> {
        let dir = self
            .root
            .join(TMP)
            .join(uuid::Uuid::new_v4().simple().to_string());
        std::fs::create_dir_all(&dir).map_err(Refusal::io)?;
        Ok(dir)
    }

    /// `e`, unpacked in `from` (a scratch folder), put in place of the version installed before, if any.
    pub fn put(&self, mut e: Installed, from: &Path) -> Result<Installed> {
        e.dir = format!("{}-{}", e.key, e.version);
        let to = self.root.join(&e.dir);
        let mut index = self.index.lock();
        let before = index.extensions.iter().position(|x| x.key == e.key);
        if to.exists() {
            std::fs::remove_dir_all(&to).map_err(Refusal::io)?;
        }
        std::fs::rename(from, &to).map_err(Refusal::io)?;
        if let Some(i) = before {
            let old = index.extensions.remove(i);
            // An update keeps whether it was turned off.
            e.enabled = old.enabled;
            if old.dir != e.dir {
                let _ = std::fs::remove_dir_all(self.root.join(&old.dir));
            }
        }
        index.extensions.push(e.clone());
        index.extensions.sort_by(|a, b| a.key.cmp(&b.key));
        self.save(&index)?;
        Ok(e)
    }

    /// Turned on or off: its files stay, the view no longer takes them.
    pub fn set_enabled(&self, key: &str, enabled: bool) -> Result<Installed> {
        let key = key.to_lowercase();
        let mut index = self.index.lock();
        let Some(e) = index.extensions.iter_mut().find(|e| e.key == key) else {
            return Err(not_installed(&key));
        };
        e.enabled = enabled;
        let out = e.clone();
        if !enabled
            && index
                .theme
                .as_deref()
                .is_some_and(|t| t.starts_with(&format!("{key}/")))
        {
            index.theme = None;
        }
        self.save(&index)?;
        Ok(out)
    }

    /// Removed, its folder with it.
    pub fn remove(&self, key: &str) -> Result<()> {
        let key = key.to_lowercase();
        let mut index = self.index.lock();
        let Some(i) = index.extensions.iter().position(|e| e.key == key) else {
            return Err(not_installed(&key));
        };
        let e = index.extensions.remove(i);
        if index
            .theme
            .as_deref()
            .is_some_and(|t| t.starts_with(&format!("{key}/")))
        {
            index.theme = None;
        }
        self.save(&index)?;
        let _ = std::fs::remove_dir_all(self.root.join(&e.dir));
        Ok(())
    }

    pub fn trusts(&self, publisher: &str) -> bool {
        let p = publisher.to_lowercase();
        self.index.lock().trusted.contains(&p)
    }

    pub fn trust(&self, publisher: &str) -> Result<()> {
        let p = publisher.to_lowercase();
        let mut index = self.index.lock();
        if !index.trusted.contains(&p) {
            index.trusted.push(p);
            index.trusted.sort();
            self.save(&index)?;
        }
        Ok(())
    }

    pub fn trusted(&self) -> Vec<String> {
        self.index.lock().trusted.clone()
    }

    /// The colour theme chosen in the view (`<extension key>/<label>`), none for Escouade's.
    pub fn theme(&self) -> Option<String> {
        self.index.lock().theme.clone()
    }

    /// Chooses a theme of an installed extension, turned on; none: back to Escouade's.
    pub fn set_theme(&self, theme: Option<String>) -> Result<()> {
        let mut index = self.index.lock();
        if let Some(t) = &theme {
            let (key, label) = t.split_once('/').unwrap_or((t, ""));
            let known = index.extensions.iter().any(|e| {
                e.key == key && e.enabled && e.contributes.themes.iter().any(|l| l == label)
            });
            if !known {
                return Err(not_installed(key));
            }
        }
        index.theme = theme;
        self.save(&index)
    }

    /// The folder of an installed extension.
    pub fn dir_of(&self, key: &str) -> Option<PathBuf> {
        self.get(key).map(|e| self.root.join(e.dir))
    }
}

fn not_installed(key: &str) -> Refusal {
    Refusal::new(
        crate::Code::NotFound,
        tr!(
            "L’extension « {key} » n’est pas installée.",
            "The extension “{key}” is not installed."
        ),
    )
}
