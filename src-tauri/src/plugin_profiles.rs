//! A plugin's profiles and its settings of each project, as the window sets them (ESC-530, issues #38 and #39;
//! docs/PLUGINS-CONTRACT-STUDY.md §2.2, §2.3 and §3.1). The host's side is `escouade_plugins::profiles`.
//!
//! - **The profiles** are made, changed and deleted on the plugin's page: their simple fields go to the list of the
//!   installed plugins, their secret fields to the keychain (or the file of ESC-499) under a name the plugin does not
//!   choose (`profiles::secret_name`, the profile's id drawn by Escouade). Deleting one erases its secrets and puts the
//!   projects that took it back to « Aucun profil » (null).
//! - **A setting of each project** (`"scope": "project"`) is set in Réglages › Projets › Plugins of the project: this
//!   machine's value, ahead of `.escouade.yml`. « Écrire dans .escouade.yml » writes it there, key by key, comments kept
//!   (`projectfile::write_plugin`), never a secret nor a profile; « Reprendre la valeur du fichier » forgets it.

use crate::core::Core;
use crate::plugin_secrets::{SecretRefused, SecretSize, SecretState};
use crate::plugins::manifest::Manifest;
use crate::plugins::profiles::{self, Kind, Profile};
use crate::projectfile;
use serde::Serialize;
use serde_json::{Map, Value};

/// The secrets of a profile, as the plugin's page shows them: kept or not, and where; never their values.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSecrets {
    pub kind: String,
    pub profile: String,
    pub secrets: Vec<SecretState>,
}

/// What deleting a profile did: the projects that took it, now with none.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Deleted {
    pub projects: Vec<String>,
}

fn not_installed() -> String {
    tr!(
        "Ce plugin n'est plus installé.",
        "This plugin is no longer installed."
    )
}

fn refused(message: String) -> SecretRefused {
    SecretRefused {
        message,
        keychain_refused: false,
    }
}

impl<R: tauri::Runtime> Core<R> {
    fn plugin_manifest(&self, id: &str) -> Result<Manifest, String> {
        self.plugins
            .get(id, crate::i18n::ui())
            .and_then(|v| v.manifest)
            .ok_or_else(not_installed)
    }

    /// The kind `kind` of the plugin `id`'s profiles.
    fn plugin_kind(&self, id: &str, kind: &str) -> Result<Kind, String> {
        profiles::kind(&self.plugin_manifest(id)?, kind)
            .ok_or_else(|| format!("{kind}: no kind of profile of {id}"))
    }

    /// The secrets of each profile of the plugin `id`: kept or not, and where; of those on several lines, their lines and
    /// bytes.
    pub fn plugin_profile_secrets(&self, id: &str) -> Result<Vec<ProfileSecrets>, String> {
        let m = self.plugin_manifest(id)?;
        let choices = self.plugins.choices(id).unwrap_or_default();
        let mut out = Vec::new();
        for kind in profiles::kinds(&m) {
            for p in choices.profiles.get(&kind.id).into_iter().flatten() {
                let secrets = kind
                    .secrets()
                    .into_iter()
                    .map(|key| {
                        let name = profiles::secret_name(id, &kind.id, &p.id, &key);
                        let value = self.secrets.get(&name).ok().flatten();
                        let multiline = kind
                            .field(&key)
                            .and_then(|f| f.get("multiline"))
                            .and_then(Value::as_bool)
                            == Some(true);
                        SecretState {
                            in_file: value.is_some() && self.kept.in_file(&name),
                            kept: value.is_some(),
                            size: value.filter(|_| multiline).map(|v| SecretSize::of(&v)),
                            key,
                        }
                    })
                    .collect();
                out.push(ProfileSecrets {
                    kind: kind.id.clone(),
                    profile: p.id.clone(),
                    secrets,
                });
            }
        }
        Ok(out)
    }

    /// « Enregistrer » of a profile's window: its simple fields (those of its kind, of their type), a name required. A new
    /// one (`profile` none) is given an id drawn here. Its id.
    pub fn plugin_profile_save(
        &self,
        id: &str,
        kind: &str,
        profile: Option<&str>,
        values: &Map<String, Value>,
    ) -> Result<String, String> {
        let kind = self.plugin_kind(id, kind)?;
        let simple = kind.simple_values(values);
        let named = simple
            .get(&kind.name_field)
            .and_then(Value::as_str)
            .is_some_and(|n| !n.trim().is_empty());
        if !named {
            return Err(tr!(
                "Un profil a un nom : donne-lui en un.",
                "A profile has a name: give it one."
            ));
        }
        let mut done: Result<String, String> = Err(not_installed());
        self.plugins
            .choose(id, |c| {
                let list = c.profiles.entry(kind.id.clone()).or_default();
                done = match profile {
                    Some(pid) => match list.iter_mut().find(|p| p.id == pid) {
                        Some(p) => {
                            p.values = simple;
                            Ok(pid.to_string())
                        }
                        None => Err(tr!(
                            "Ce profil n'existe plus.",
                            "This profile no longer exists."
                        )),
                    },
                    None => {
                        let new = profiles::new_id();
                        list.push(Profile {
                            id: new.clone(),
                            values: simple,
                        });
                        Ok(new)
                    }
                };
            })
            .map_err(|e| e.to_string())?;
        done
    }

    /// « Garder » (`value`; empty: « Oublier ») of a secret field of a profile, under its own name in the keychain.
    pub fn plugin_profile_secret(
        &self,
        id: &str,
        kind: &str,
        profile: &str,
        key: &str,
        value: &str,
        in_file: bool,
    ) -> Result<(), SecretRefused> {
        let k = self.plugin_kind(id, kind).map_err(refused)?;
        if !k.has_secret(key) {
            return Err(refused(format!(
                "{key}: not a secret of the profiles {kind}"
            )));
        }
        let exists = self.plugins.choices(id).is_some_and(|c| {
            c.profiles
                .get(kind)
                .is_some_and(|l| l.iter().any(|p| p.id == profile))
        });
        if !exists || !profiles::is_id(profile) {
            return Err(refused(tr!(
                "Ce profil n'existe plus.",
                "This profile no longer exists."
            )));
        }
        let name = profiles::secret_name(id, kind, profile, key);
        self.plugin_secret_set_named(id, key, &name, value, in_file)
    }

    /// « Supprimer le profil »: its secrets erased, the projects that took it with none (null), a default that named it
    /// forgotten. The projects that took it.
    pub fn plugin_profile_delete(
        &self,
        id: &str,
        kind: &str,
        profile: &str,
    ) -> Result<Deleted, String> {
        let k = self.plugin_kind(id, kind)?;
        let m = self.plugin_manifest(id)?;
        let choices = self.plugins.choices(id).ok_or_else(not_installed)?;
        if !choices
            .profiles
            .get(kind)
            .is_some_and(|l| l.iter().any(|p| p.id == profile))
        {
            return Err(tr!(
                "Ce profil n'existe plus.",
                "This profile no longer exists."
            ));
        }
        // Its secrets: those of its kind, and any name kept under its own prefix.
        let prefix = profiles::secret_name(id, kind, profile, "");
        let mut names: Vec<String> = k
            .secrets()
            .iter()
            .map(|key| profiles::secret_name(id, kind, profile, key))
            .collect();
        for n in &choices.secret_names {
            if n.starts_with(&prefix) && !names.contains(n) {
                names.push(n.clone());
            }
        }
        for n in &names {
            if let Err(e) = self.secrets.delete(n) {
                log::warn!(
                    "plugin {id}: a secret of a profile deleted stays in the keychain: {e:#}"
                );
            }
        }
        let keys: Vec<(String, bool)> = profiles::declared(&m)
            .iter()
            .filter(|d| d.get("type").and_then(Value::as_str) == Some("profile"))
            .filter(|d| d.get("of").and_then(Value::as_str) == Some(kind))
            .filter_map(|d| {
                let key = d.get("key")?.as_str()?.to_string();
                Some((key, profiles::of_project(d)))
            })
            .collect();
        let mut projects = Vec::new();
        self.plugins
            .choose(id, |c| {
                if let Some(list) = c.profiles.get_mut(kind) {
                    list.retain(|p| p.id != profile);
                }
                c.secret_names.retain(|n| !names.contains(n));
                for (project, values) in c.project_settings.iter_mut() {
                    for (key, of_project) in &keys {
                        if *of_project && values.get(key).and_then(Value::as_str) == Some(profile) {
                            values.insert(key.clone(), Value::Null);
                            if !projects.contains(project) {
                                projects.push(project.clone());
                            }
                        }
                    }
                }
                for (key, of_project) in &keys {
                    if !*of_project && c.settings.get(key).and_then(Value::as_str) == Some(profile)
                    {
                        c.settings.remove(key);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Deleted { projects })
    }

    /// The setting `key` of each project (`"scope": "project"`) of the plugin `id`.
    fn project_setting(&self, id: &str, key: &str) -> Result<Map<String, Value>, String> {
        profiles::setting(&self.plugin_manifest(id)?, key)
            .filter(profiles::of_project)
            .ok_or_else(|| format!("{key}: not a setting of each project of {id}"))
    }

    /// A setting of each project set on this machine (`value`), ahead of the project's `.escouade.yml`; none: forgotten
    /// (« Reprendre la valeur du fichier », or « Par défaut » of a profile). A `profile` setting takes a profile of its
    /// kind, or null (« Aucun profil »).
    pub fn plugin_project_set(
        &self,
        id: &str,
        project: &str,
        key: &str,
        value: Option<Value>,
    ) -> Result<(), String> {
        let decl = self.project_setting(id, key)?;
        self.project(project).map_err(|e| e.to_string())?;
        let value = match value {
            None => None,
            Some(v) if decl.get("type").and_then(Value::as_str) == Some("profile") => {
                let kind = decl.get("of").and_then(Value::as_str).unwrap_or_default();
                let fits = match &v {
                    Value::Null => true,
                    Value::String(pid) => self.plugins.choices(id).is_some_and(|c| {
                        c.profiles
                            .get(kind)
                            .is_some_and(|l| l.iter().any(|p| &p.id == pid))
                    }),
                    _ => false,
                };
                if !fits {
                    return Err(tr!(
                        "Ce profil n'existe plus.",
                        "This profile no longer exists."
                    ));
                }
                Some(v)
            }
            Some(Value::Null) => None,
            Some(v) if profiles::fits(&decl, &v) => Some(v),
            Some(_) => return Err(format!("{key}: a value of its type")),
        };
        self.plugins
            .choose(id, |c| {
                let values = c.project_settings.entry(project.to_string()).or_default();
                match value {
                    Some(v) => {
                        values.insert(key.to_string(), v);
                    }
                    None => {
                        values.remove(key);
                    }
                }
                c.project_settings.retain(|_, v| !v.is_empty());
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// « Écrire dans .escouade.yml » of a setting of each project: this machine's value written into the project's file
    /// (`plugins.<id>.<key>`, its comments kept), then forgotten here: the file's is the project's again. Never a secret,
    /// a profile, a value imported from a file, nor what its author keeps off the file (`"repo": false`).
    pub fn plugin_project_write(
        self: &std::sync::Arc<Self>,
        id: &str,
        project: &str,
        key: &str,
    ) -> Result<crate::model::Project, String> {
        let decl = self.project_setting(id, key)?;
        if !profiles::from_repo(&decl) {
            return Err(tr!(
                "Ce réglage reste sur cette machine : il ne s'écrit pas dans .escouade.yml.",
                "This setting stays on this machine: it is not written into .escouade.yml."
            ));
        }
        let value = self
            .plugins
            .choices(id)
            .and_then(|c| c.project_settings.get(project)?.get(key).cloned())
            .filter(|v| !v.is_null())
            .ok_or_else(|| format!("{key}: no value kept on this machine"))?;
        let p = self.project(project).map_err(|e| e.to_string())?;
        let path = std::path::Path::new(&p.path).join(projectfile::FILE);
        let existing = std::fs::read_to_string(&path).ok();
        let text = projectfile::write_plugin(existing.as_deref(), id, key, &value)
            .map_err(|e| format!("{e:#}"))?;
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        self.plugin_project_set(id, project, key, None)?;
        // The file read again: what the window marks follows it.
        if self.read_shared(project, false).is_none() {
            if let Ok(p) = self.project(project) {
                self.hub.emit(crate::model::UiEvent::Project { project: p });
            }
        }
        self.project(project).map_err(|e| e.to_string())
    }
}
