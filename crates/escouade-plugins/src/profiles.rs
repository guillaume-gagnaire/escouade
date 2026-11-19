//! A plugin's profiles and the settings of each project (ESC-530, issues #38 and #39; docs/PLUGINS-CONTRACT-STUDY.md
//! §2.2 to §2.4 and §3.1, direction B chosen on 2026-11-17).
//!
//! - **A profile** (`contributes.profiles`) is a set of fields the user makes as many times as they want on the plugin's
//!   page (a server, its credentials, its configuration), and that each project chooses. Its id is **drawn by Escouade**
//!   (`new_id`): never the plugin's nor the repository's text. Its simple fields are kept with the list of the installed
//!   plugins (`Choices::profiles`); its secret fields go to the keychain (or the file of ESC-499) under
//!   `plugin:<id>:profile:<kind>:<profile>:<key>` (`secret_name`), whose names Escouade lists (`Choices::secret_names`)
//!   since the keychain cannot list by prefix.
//! - **A setting of each project** (`"scope": "project"`) is no longer on the plugin's page: its value of this machine is
//!   kept by project (`Choices::project_settings`), ahead of the project's `.escouade.yml`, itself ahead of the manifest's
//!   `default` (`settings_for`). A `profile` setting holds the id of the profile chosen, or null: a `.escouade.yml` never
//!   chooses one, it may only propose one by its name (`proposed`), which the window says and the user takes or not.
//!
//! The ranks of a setting on `main`: 1 the manifest's default, 3 the project's `.escouade.yml`, 4 this machine's value. The
//! team's ranks (2 recommended, 5 enforced) and the managed device's (6) come with the team server of the 2.3.

use crate::manifest::Manifest;
use crate::Choices;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// A profile the user made: its id drawn by Escouade, and its simple fields (never a secret field: those are kept under
/// `secret_name`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    #[serde(default)]
    pub values: Map<String, Value>,
}

/// A kind of profile, as the manifest declares it (`contributes.profiles[]`, read and checked by ESC-525).
#[derive(Debug, Clone, PartialEq)]
pub struct Kind {
    pub id: String,
    /// The simple field a profile is named by.
    pub name_field: String,
    /// Its fields, in the manifest's order.
    pub fields: Vec<Map<String, Value>>,
}

impl Kind {
    /// The keys of its secret fields (`"secret": true`).
    pub fn secrets(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter(|f| is_secret(f))
            .filter_map(|f| f.get("key").and_then(Value::as_str).map(str::to_string))
            .collect()
    }

    /// Its field `key`.
    pub fn field(&self, key: &str) -> Option<&Map<String, Value>> {
        self.fields
            .iter()
            .find(|f| f.get("key").and_then(Value::as_str) == Some(key))
    }

    /// Whether `key` is one of its secret fields.
    pub fn has_secret(&self, key: &str) -> bool {
        self.field(key).is_some_and(is_secret)
    }

    /// The name of the profile `p`: its `nameField`, else its id.
    pub fn name_of(&self, p: &Profile) -> String {
        p.values
            .get(&self.name_field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map_or_else(|| p.id.clone(), str::to_string)
    }

    /// Its simple fields as given (`values`), held to those it declares with a value of their type: what a profile keeps.
    pub fn simple_values(&self, values: &Map<String, Value>) -> Map<String, Value> {
        let mut out = Map::new();
        for f in self.fields.iter().filter(|f| !is_secret(f)) {
            let Some(key) = f.get("key").and_then(Value::as_str) else {
                continue;
            };
            match values.get(key) {
                Some(v) if fits(f, v) => {
                    out.insert(key.to_string(), v.clone());
                }
                _ => {}
            }
        }
        out
    }
}

fn is_secret(f: &Map<String, Value>) -> bool {
    f.get("secret").and_then(Value::as_bool) == Some(true)
}

/// The kinds of profile of a manifest.
pub fn kinds(m: &Manifest) -> Vec<Kind> {
    m.contributes
        .get("profiles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|k| {
            Some(Kind {
                id: k.get("id")?.as_str()?.to_string(),
                name_field: k.get("nameField")?.as_str()?.to_string(),
                fields: k
                    .get("fields")?
                    .as_array()?
                    .iter()
                    .filter_map(|f| f.as_object().cloned())
                    .collect(),
            })
        })
        .collect()
}

/// Its kind `id`.
pub fn kind(m: &Manifest, id: &str) -> Option<Kind> {
    kinds(m).into_iter().find(|k| k.id == id)
}

/// The length of a profile's id: 16 hexadecimal digits, drawn by Escouade.
pub const ID_LEN: usize = 16;

/// A new profile's id: random, drawn by Escouade, never chosen by the plugin nor by a repository (§2.3).
pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..ID_LEN].to_string()
}

/// Whether `s` is an id `new_id` draws: what a name in the keychain may hold, nothing else.
pub fn is_id(s: &str) -> bool {
    s.len() == ID_LEN
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The name in the keychain of the secret field `key` of the profile `profile` of the kind `kind` of the plugin `plugin`
/// (§2.3): under the plugin's own prefix, so that only it reads it.
pub fn secret_name(plugin: &str, kind: &str, profile: &str, key: &str) -> String {
    format!("plugin:{plugin}:profile:{kind}:{profile}:{key}")
}

/// The settings a manifest declares (`contributes.settings`).
pub fn declared(m: &Manifest) -> Vec<Map<String, Value>> {
    m.contributes
        .get("settings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_object().cloned())
        .collect()
}

/// Whether a setting is of each project (`"scope": "project"`).
pub fn of_project(decl: &Map<String, Value>) -> bool {
    decl.get("scope").and_then(Value::as_str) == Some("project")
}

/// The setting `key` a manifest declares.
pub fn setting(m: &Manifest, key: &str) -> Option<Map<String, Value>> {
    declared(m)
        .into_iter()
        .find(|d| d.get("key").and_then(Value::as_str) == Some(key))
}

/// Whether a setting may come from `.escouade.yml` or be written there (choice n° 2): a simple setting whose author did not
/// keep it off the file (`"repo": false`), never one imported from a file, never a profile.
pub fn from_repo(decl: &Map<String, Value>) -> bool {
    decl.get("repo").and_then(Value::as_bool) != Some(false)
        && decl.get("import").is_none()
        && matches!(
            decl.get("type").and_then(Value::as_str),
            Some("string" | "number" | "boolean" | "select")
        )
}

/// Whether `value` is one of the setting's (or field's) type: a `select` among its options when it lists them.
pub fn fits(decl: &Map<String, Value>, value: &Value) -> bool {
    match decl.get("type").and_then(Value::as_str) {
        Some("string") => value.is_string(),
        Some("number") => value.is_number(),
        Some("boolean") => value.is_boolean(),
        Some("select") => value.as_str().is_some_and(|v| {
            decl.get("options")
                .and_then(Value::as_array)
                .is_none_or(|options| {
                    options.iter().any(|o| {
                        o.as_str()
                            .or_else(|| o.get("value").and_then(Value::as_str))
                            == Some(v)
                    })
                })
        }),
        _ => false,
    }
}

/// The profile `id` among the user's, with its kind's id.
pub fn find<'a>(choices: &'a Choices, id: &str) -> Option<(&'a str, &'a Profile)> {
    choices
        .profiles
        .iter()
        .find_map(|(kind, list)| list.iter().find(|p| p.id == id).map(|p| (kind.as_str(), p)))
}

/// Whether the profile `id` is one of the kind `kind`.
fn exists(choices: &Choices, kind: &str, id: &str) -> bool {
    choices
        .profiles
        .get(kind)
        .is_some_and(|l| l.iter().any(|p| p.id == id))
}

/// A profile's id of the kind `kind` held by `v`, if it still is one; else null.
fn chosen(choices: &Choices, kind: &str, v: Option<&Value>) -> Value {
    match v.and_then(Value::as_str) {
        Some(id) if exists(choices, kind, id) => json!(id),
        _ => Value::Null,
    }
}

/// The profile of the kind `kind` a project takes when it chooses none: the one of a `profile` setting of the app's scope
/// (« Profil par défaut »), if any.
fn default_profile(m: &Manifest, choices: &Choices, kind: &str) -> Value {
    declared(m)
        .iter()
        .filter(|d| !of_project(d))
        .filter(|d| d.get("type").and_then(Value::as_str) == Some("profile"))
        .filter(|d| d.get("of").and_then(Value::as_str) == Some(kind))
        .filter_map(|d| d.get("key").and_then(Value::as_str))
        .map(|key| chosen(choices, kind, choices.settings.get(key)))
        .find(|v| !v.is_null())
        .unwrap_or(Value::Null)
}

/// The settings the plugin works with (`settings/get`), for the project `project` when it names one (`active`: the plugin
/// is on there, `active_in`; `file`: what its `.escouade.yml` gives the plugin, `plugins.<id>`).
///
/// - A setting of the app's scope: the user's value, then over it the file's (as before, PLG-14).
/// - A setting of each project, by rank: this machine's value for the project (4), the file's (3), the manifest's
///   `default` (1). Only where the plugin is on: elsewhere, the default alone.
/// - A `profile` setting: the id of the profile chosen, never from the file; a project that chose none takes the
///   profile of an app's `profile` setting of the same kind, else null. A profile gone is null.
pub fn settings_for(
    m: Option<&Manifest>,
    choices: &Choices,
    project: Option<&str>,
    active: bool,
    file: Option<&Map<String, Value>>,
) -> Map<String, Value> {
    let Some(m) = m else {
        return choices.settings.clone();
    };
    let decls = declared(m);
    let mut mine = choices.settings.clone();
    // A value kept on the plugin's page before a setting became a project's is not a project's.
    for d in decls.iter().filter(|d| of_project(d)) {
        if let Some(k) = d.get("key").and_then(Value::as_str) {
            mine.remove(k);
        }
    }
    let here = project.filter(|_| active);
    let mut out = match (here, file) {
        (Some(_), Some(file)) => super::project_settings(m, &mine, file),
        _ => mine,
    };
    let local = here.and_then(|p| choices.project_settings.get(p));
    for d in &decls {
        let Some(key) = d.get("key").and_then(Value::as_str) else {
            continue;
        };
        let profile = d.get("type").and_then(Value::as_str) == Some("profile");
        let kind = d.get("of").and_then(Value::as_str).unwrap_or_default();
        if !of_project(d) {
            if profile {
                let v = chosen(choices, kind, choices.settings.get(key));
                out.insert(key.to_string(), v);
            }
            continue;
        }
        let mine = local.and_then(|l| l.get(key));
        if profile {
            let v = match (here, mine) {
                (None, _) => Value::Null,
                (Some(_), Some(v)) => chosen(choices, kind, Some(v)),
                (Some(_), None) => default_profile(m, choices, kind),
            };
            out.insert(key.to_string(), v);
        } else if let Some(v) = mine.filter(|v| fits(d, v)) {
            out.insert(key.to_string(), v.clone());
        } else if !out.contains_key(key) {
            if let Some(v) = d.get("default").filter(|v| fits(d, v)) {
                out.insert(key.to_string(), v.clone());
            }
        }
    }
    out
}

/// What the project's `.escouade.yml` proposes for the `profile` settings of a plugin: by setting, the name it gives (a
/// text). A proposal, never a choice (§2.3): the window says it, and the user chooses.
pub fn proposed(m: &Manifest, file: &Map<String, Value>) -> Vec<(String, String)> {
    declared(m)
        .iter()
        .filter(|d| of_project(d) && d.get("type").and_then(Value::as_str) == Some("profile"))
        .filter_map(|d| {
            let key = d.get("key")?.as_str()?;
            let name = file.get(key)?.as_str()?.trim();
            (!name.is_empty()).then(|| (key.to_string(), name.to_string()))
        })
        .collect()
}
