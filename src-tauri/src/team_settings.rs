//! The team's `.escouade.yml` in the app (TS-13, `docs/TEAM-SERVER.md` §6.1, §6.2, §6.5 and §6.6).
//!
//! `escouade-team::settings` holds the file on the server: signed by an administrator, chained, under the quorum for
//! what is sensitive, refused when it holds what never enters. The core gives it:
//!
//! - **the sort of each key** ([`CoreKeys`]): the single list of the policies (`escouade_policy::list`, each key with its
//!   sort), then the keys of the repository's `.escouade.yml` (`projectfile::KEYS`: a command is executable, the rest a
//!   value), and the plugins to install (executable);
//! - **the file as YAML** ([`parse`], [`to_yaml`]): what the administrator writes and reads in the window, its parts
//!   flattened into the list's keys (`permissions.modes`, or `permissions:` then `modes:`);
//! - **where it applies** ([`Layers`], `Core::apply_team_settings`): the policies' keys go to the locks (rank 5 for
//!   `enforced`, rank 2 for `defaults`), the keys of a project to the team's projects alone (rank 2 under the
//!   repository's file, rank 5 over it); what runs applies once this device accepted it, at its fingerprint.

use std::collections::BTreeMap;

use escouade_team::settings::{Classify, Sort, TeamFile, View};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::locks::TeamPolicies;
use crate::projectfile::{self, Node, N};

/// The core's list of the keys a team's file may name.
pub struct CoreKeys;

pub static CORE_KEYS: CoreKeys = CoreKeys;

/// The keys that install a plugin: proposed alone, each device's gesture.
const INSTALL: &str = "plugins.install";

impl Classify for CoreKeys {
    fn sort(&self, key: &str) -> Option<Sort> {
        use escouade_policy::list::Sort as P;
        if let Some(d) = escouade_policy::list::def(key) {
            return Some(match d.sort {
                P::Restrict => Sort::Restrict,
                P::Value => Sort::Value,
                P::Sensitive => Sort::Sensitive,
                P::Executable => Sort::Executable,
            });
        }
        if key == INSTALL {
            return Some(Sort::Executable);
        }
        if key == projectfile::RECOMMENDED {
            return Some(Sort::Value);
        }
        let k = projectfile::key_of(key)?;
        Some(if k.command {
            Sort::Executable
        } else {
            Sort::Value
        })
    }
}

/// Whether a key is one of the policies (the locks hold it), not of a project's file.
pub fn is_policy(key: &str) -> bool {
    escouade_policy::list::def(key).is_some()
}

// ---------- the YAML ----------

fn json_of(node: &Node) -> Value {
    match &node.v {
        N::Scalar { null: true, .. } => Value::Null,
        N::Scalar {
            text, quoted: true, ..
        } => Value::String(text.clone()),
        N::Scalar { text, .. } => match text.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            t => t
                .parse::<i64>()
                .map(Value::from)
                .or_else(|_| t.parse::<f64>().map(Value::from))
                .unwrap_or_else(|_| Value::String(t.to_owned())),
        },
        N::Seq(items) => Value::Array(items.iter().map(json_of).collect()),
        N::Map(entries) => Value::Object(
            entries
                .iter()
                .map(|(k, _, v)| (k.clone(), json_of(v)))
                .collect::<Map<String, Value>>(),
        ),
        N::Other => Value::Null,
    }
}

/// A part's tree flattened into the list's keys: a key the list knows takes its whole value; a mapping that is not
/// one is walked into. A key nobody knows is kept (what never enters is checked on it), and said.
fn flatten(
    node: &Node,
    prefix: &str,
    out: &mut BTreeMap<String, Value>,
    warnings: &mut Vec<String>,
) {
    let N::Map(entries) = &node.v else {
        return;
    };
    for (k, _, v) in entries {
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        if CORE_KEYS.sort(&path).is_some() {
            out.insert(path, json_of(v));
        } else if matches!(v.v, N::Map(_)) {
            flatten(v, &path, out, warnings);
        } else {
            warnings.push(tr!(
                "« {path} » n’est pas une clé connue : elle ne s’appliquera pas.",
                "“{path}” is not a known key: it will not apply."
            ));
            out.insert(path, json_of(v));
        }
    }
}

/// The team's file, from its YAML: the file, and what was left out or will not apply. Err: it cannot be read.
pub fn parse(text: &str) -> Result<(TeamFile, Vec<String>), String> {
    let tree = projectfile::tree(text).map_err(|w| w.text)?;
    let mut file = TeamFile {
        version: escouade_team::settings::FORMAT,
        ..Default::default()
    };
    let mut warnings = Vec::new();
    let N::Map(entries) = &tree.v else {
        return Ok((file, warnings));
    };
    for (k, _, v) in entries {
        match k.as_str() {
            "version" => match json_of(v).as_u64() {
                Some(n) if n == u64::from(escouade_team::settings::FORMAT) => {}
                _ => {
                    return Err(tr!(
                        "version : seule la version 1 du fichier d’équipe est lue.",
                        "version: only version 1 of the team’s file is read."
                    ))
                }
            },
            "defaults" => flatten(v, "", &mut file.defaults, &mut warnings),
            "enforced" => flatten(v, "", &mut file.enforced, &mut warnings),
            "proposed" => flatten(v, "", &mut file.proposed, &mut warnings),
            "reasons" => {
                if let Value::Object(m) = json_of(v) {
                    for (key, why) in m {
                        file.reasons.insert(
                            key,
                            why.as_str().map(str::to_owned).unwrap_or_default(),
                        );
                    }
                }
            }
            "quorum" => match json_of(v) {
                Value::Bool(b) => file.quorum = Some(b),
                Value::Null => file.quorum = None,
                _ => {
                    return Err(tr!(
                        "quorum : true ou false.",
                        "quorum: true or false."
                    ))
                }
            },
            "signers" => {
                for s in json_of(v).as_array().into_iter().flatten() {
                    let mut id = [0u8; 16];
                    match s
                        .as_str()
                        .map(|t| hex::decode_to_slice(t.trim(), &mut id))
                    {
                        Some(Ok(())) => file.signers.push(escouade_team::crypto::MemberId(id)),
                        _ => {
                            return Err(tr!(
                                "signers : les identifiants des membres (32 caractères hexadécimaux).",
                                "signers: the ids of the members (32 hexadecimal characters)."
                            ))
                        }
                    }
                }
            }
            other => warnings.push(tr!(
                "« {other} » n’est pas une partie du fichier d’équipe (defaults, enforced, proposed, reasons, quorum, signers).",
                "“{other}” is not a part of the team’s file (defaults, enforced, proposed, reasons, quorum, signers)."
            )),
        }
    }
    Ok((file, warnings))
}

/// The file as YAML, each key on its line with its value in JSON (which YAML reads): what the window starts the next
/// version from. `parse(to_yaml(f)) == f`.
pub fn to_yaml(file: &TeamFile) -> String {
    let mut out = String::from("version: 1\n");
    for (name, part) in [
        ("defaults", &file.defaults),
        ("enforced", &file.enforced),
        ("proposed", &file.proposed),
    ] {
        if part.is_empty() {
            continue;
        }
        out.push_str(name);
        out.push_str(":\n");
        for (k, v) in part {
            out.push_str(&format!("  {k}: {v}\n"));
        }
    }
    if !file.reasons.is_empty() {
        out.push_str("reasons:\n");
        for (k, why) in &file.reasons {
            out.push_str(&format!("  {k}: {}\n", Value::String(why.clone())));
        }
    }
    if let Some(q) = file.quorum {
        out.push_str(&format!("quorum: {q}\n"));
    }
    if !file.signers.is_empty() {
        let ids: Vec<Value> = file
            .signers
            .iter()
            .map(|m| Value::String(m.to_string()))
            .collect();
        out.push_str(&format!("signers: {}\n", Value::Array(ids)));
    }
    out
}

// ---------- what applies ----------

/// What a team sets on its projects, of the keys of a project's file (`projectfile::KEYS`).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layers {
    /// Rank 2: under the repository's file and the device.
    pub defaults: BTreeMap<String, Value>,
    /// Rank 5: over everything but the managed device.
    pub enforced: BTreeMap<String, Value>,
    /// The commands this device accepted: applied as recommended (rank 2).
    pub accepted: BTreeMap<String, Value>,
}

impl Layers {
    pub fn is_empty(&self) -> bool {
        self.defaults.is_empty() && self.enforced.is_empty() && self.accepted.is_empty()
    }

    /// What a project's reading compares, to read the file again when the team changed it.
    pub fn fingerprint(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        projectfile::fingerprint(&BTreeMap::from([
            (
                "d".to_owned(),
                Value::from(Map::from_iter(self.defaults.clone())),
            ),
            (
                "e".to_owned(),
                Value::from(Map::from_iter(self.enforced.clone())),
            ),
            (
                "a".to_owned(),
                Value::from(Map::from_iter(self.accepted.clone())),
            ),
        ]))
    }
}

/// What the version in force of a team gives the locks (policies' keys) and its projects (projects' keys).
pub fn split(id: &str, name: &str, view: &View) -> (Option<TeamPolicies>, Layers) {
    let Some(applied) = view.applied else {
        return (None, Layers::default());
    };
    let e = &view.effective;
    let version = view.versions.iter().find(|v| v.number == applied);
    let pick = |m: &BTreeMap<String, Value>, policy: bool| -> BTreeMap<String, Value> {
        m.iter()
            .filter(|(k, _)| is_policy(k) == policy && CORE_KEYS.sort(k).is_some())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };
    let policies = TeamPolicies {
        id: id.to_owned(),
        team: name.to_owned(),
        signed: version.map(|v| (v.by.clone(), v.at)),
        recommended: pick(&e.defaults, true),
        enforced: pick(&e.enforced, true),
    };
    let layers = Layers {
        defaults: pick(&e.defaults, false),
        enforced: pick(&e.enforced, false),
        accepted: pick(&e.accepted, false),
    };
    (Some(policies), layers)
}

impl crate::core::Core {
    /// The version in force of the team `id` (None: the device left it, or the team has none): its policies to the
    /// locks, its keys of a project to each of its projects, read again at once.
    pub(crate) fn apply_team_settings(
        self: &std::sync::Arc<Self>,
        id: &str,
        name: &str,
        view: Option<&View>,
    ) {
        let (policies, layers) = view.map(|v| split(id, name, v)).unwrap_or_default();
        self.set_team_policies_of(id, policies);
        let changed = {
            let mut all = self.team_layers.write();
            let before = all.get(id).cloned().unwrap_or_default();
            if layers.is_empty() {
                all.remove(id);
            } else {
                all.insert(id.to_owned(), layers.clone());
            }
            before != layers
        };
        if !changed {
            return;
        }
        let projects: Vec<String> = self
            .projects
            .read()
            .iter()
            .filter(|p| p.team.as_deref() == Some(id))
            .map(|p| p.id.clone())
            .collect();
        for p in projects {
            self.read_shared(&p, false);
        }
    }

    /// What the team of a project sets on it (its keys of a project's file); none for a project of no team.
    pub(crate) fn team_layers_for(&self, project_id: &str) -> Option<(String, Layers)> {
        let team = self
            .projects
            .read()
            .iter()
            .find(|p| p.id == project_id)?
            .team
            .clone()?;
        let layers = self.team_layers.read().get(&team).cloned()?;
        let name = self
            .team_policies
            .read()
            .get(&team)
            .map(|t| t.team.clone())
            .unwrap_or(team);
        Some((name, layers))
    }

    /// « Projet de l'équipe »: the project `project_id` made a project of the team `team` (or of none). What the team
    /// imposes and recommends holds for it from now on; the locks and its file are read again.
    pub(crate) fn set_project_team(
        self: &std::sync::Arc<Self>,
        project_id: &str,
        team: Option<String>,
    ) -> anyhow::Result<crate::model::Project> {
        let project = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(|| anyhow::anyhow!(tr!("Projet introuvable.", "Project not found.")))?;
            p.team = team.filter(|t| !t.trim().is_empty());
            p.shared.seen = None;
            p.clone()
        };
        self.hub.emit(crate::model::UiEvent::Project {
            project: project.clone(),
        });
        self.request_save();
        self.settle_locks();
        self.read_shared(project_id, false);
        self.project(project_id)
    }
}

#[cfg(test)]
#[path = "team_settings_tests.rs"]
mod tests;
