//! `extensions.allowed`, VS Code's rule (« Configure allowed extensions »): an object whose keys are `*`, a publisher
//! (`dracula-theme`) or an extension (`dracula-theme.theme-dracula`), and whose values are `true`, `false`, `"stable"`
//! (no pre-release) or, for an extension, the list of its versions allowed (`["2.25.1"]`; a `@platform` after a version
//! is read as the version alone). The most precise key that matches says it: the extension's, else its publisher's,
//! else `*`. An object with no key, or no object, allows everything; an object with keys allows only what it names.
//!
//! Escouade has four layers of it (the settings of this device, each open project's `.escouade.yml`, the team, the
//! managed device): an extension must be allowed by each layer that sets one. A layer says who it is, the refusal
//! names it.

use crate::{Code, ExtId, Refusal};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// What a key of the object says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "versions")]
pub enum Rule {
    Allow,
    Deny,
    /// Stable versions alone.
    Stable,
    /// These versions alone (of an extension).
    Versions(Vec<String>),
}

/// One layer's object, its keys in lower case.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Rules(pub BTreeMap<String, Rule>);

impl Rules {
    /// The object of a layer; what cannot be read is said, the rest kept. `null` is no rule (everything allowed).
    pub fn parse(v: &Value) -> (Rules, Vec<String>) {
        let mut out = BTreeMap::new();
        let mut warnings = Vec::new();
        let Some(map) = v.as_object() else {
            if !v.is_null() {
                warnings.push(tr!(
                    "extensions.allowed : un objet est attendu, comme {{\"*\": true}}",
                    "extensions.allowed: an object is expected, such as {{\"*\": true}}"
                ));
            }
            return (Rules::default(), warnings);
        };
        for (k, v) in map {
            let key = k.trim().to_lowercase();
            let id_like = key == "*"
                || crate::is_name(&key)
                || key
                    .split_once('.')
                    .is_some_and(|(a, b)| crate::is_name(a) && crate::is_name(b));
            if !id_like {
                warnings.push(tr!(
                    "extensions.allowed : « {k} » n’est ni *, ni un éditeur, ni une extension (éditeur.nom)",
                    "extensions.allowed: “{k}” is neither *, a publisher nor an extension (publisher.name)"
                ));
                continue;
            }
            let rule = match v {
                Value::Bool(true) => Rule::Allow,
                Value::Bool(false) => Rule::Deny,
                Value::String(s) if s == "stable" => Rule::Stable,
                Value::Array(a) if key.contains('.') => {
                    let versions: Vec<String> = a
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|s| s.split('@').next().unwrap_or(s).trim().to_owned())
                        .filter(|s| crate::is_version(s))
                        .collect();
                    if versions.len() != a.len() {
                        warnings.push(tr!(
                            "extensions.allowed : « {k} » : une liste de versions est attendue",
                            "extensions.allowed: “{k}”: a list of versions is expected"
                        ));
                    }
                    Rule::Versions(versions)
                }
                _ => {
                    warnings.push(tr!(
                        "extensions.allowed : « {k} » : true, false, \"stable\" ou une liste de versions est attendu",
                        "extensions.allowed: “{k}”: true, false, \"stable\" or a list of versions is expected"
                    ));
                    continue;
                }
            };
            out.insert(key, rule);
        }
        (Rules(out), warnings)
    }

    /// Whether it says anything: an empty object allows everything.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The rule that says it for `id`: its own key, its publisher's, `*`.
    pub fn rule_for(&self, id: &ExtId) -> Option<&Rule> {
        self.0
            .get(&id.key())
            .or_else(|| self.0.get(&id.namespace.to_lowercase()))
            .or_else(|| self.0.get("*"))
    }

    /// Whether `id` may be installed: at `version` (and whether it is a pre-release) when known; before the version is
    /// known, an extension whose versions are listed passes (its version is checked later, before the download).
    pub fn allows(&self, id: &ExtId, version: Option<(&str, bool)>) -> bool {
        if self.is_empty() {
            return true;
        }
        match self.rule_for(id) {
            None | Some(Rule::Deny) => false,
            Some(Rule::Allow) => true,
            Some(Rule::Stable) => version.is_none_or(|(_, pre)| !pre),
            Some(Rule::Versions(list)) => version.is_none_or(|(v, _)| list.iter().any(|x| x == v)),
        }
    }
}

/// A layer that sets a list, and who it is, as a refusal says it (« par les réglages de ce poste », « par le
/// .escouade.yml de Atlas », « par l’équipe Acme », « par la stratégie de ce poste »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub by: String,
    /// Whether it is imposed (the team's, the managed device's): the page greys what it says.
    pub imposed: bool,
    pub rules: Rules,
}

/// `id` (at `version`) checked against each layer, the strongest first (as they are given): the first that refuses
/// says it.
pub fn check(layers: &[Layer], id: &ExtId, version: Option<(&str, bool)>) -> Result<(), Refusal> {
    for layer in layers {
        if !layer.rules.allows(id, version) {
            let by = &layer.by;
            let message = match version {
                Some((v, _)) if layer.rules.allows(id, None) => tr!(
                    "La version {v} de « {id} » n’est pas permise {by}.",
                    "Version {v} of “{id}” is not allowed {by}."
                ),
                _ => tr!(
                    "L’extension « {id} » n’est pas permise {by}.",
                    "The extension “{id}” is not allowed {by}."
                ),
            };
            return Err(Refusal::new(Code::NotAllowed, message));
        }
    }
    Ok(())
}
