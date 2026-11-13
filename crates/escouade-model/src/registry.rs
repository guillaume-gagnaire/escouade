//! The models a harness lists for an account (`engine::registry::ToldModel`), which the composer's menu offers, and
//! the menu Claude Code's `initialize` makes (`engine::claude_catalog`).

use serde::Serialize;

use crate::model::ModelInfo;

/// A model as its harness lists it for the account (Codex: `model/list`), which the composer's
/// menu of its agents offers in place of the declared ones (`ModelDecl`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToldModel {
    /// The id its harness takes.
    pub id: String,
    /// Its name in the menu (`GPT-6-Luna`).
    pub label: String,
    /// The efforts it takes, in its harness's order (none: no menu « Effort »).
    pub efforts: Vec<String>,
    pub default_effort: Option<String>,
    /// It reads images.
    pub images: bool,
    /// The one its harness runs unless told otherwise: the model of a new agent of it.
    pub default: bool,
    /// The provider it comes from, when its harness has several (OpenCode: `Ollama (local)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// What it costs beside the others, as its harness bills it (Copilot: `billing.multiplier`,
    /// `1` the base rate, written `×3` beside it in the menu).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<String>,
    /// Its tier of price, when its harness tells no multiplier (Copilot: `low`, `medium`,
    /// `high`, `very_high`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    /// Not listed by its harness, but the model the account's config names (Codex's `model` of
    /// its `config.toml`, ESC-186): offered after the listed ones, marked as coming from there.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_config: bool,
}

/// The composer's menu of the agents of an account, as its `claude` told its models
/// (`initialize`, ESC-124): in its order and by the names of its `/model`, each with its
/// efforts; `default` (Claude Code's choice, the same model as another) is not one, but marks
/// the one it runs. Empty when it tells no names (a list read before ESC-124): the fixed one
/// serves.
pub fn claude_catalog(told: &[ModelInfo]) -> Vec<ToldModel> {
    if told.iter().all(|m| m.display_name.is_empty()) {
        return Vec::new();
    }
    let default = told
        .iter()
        .find(|m| m.value == "default")
        .map(|m| m.resolved_model.as_str());
    let mut marked = false;
    let mut out = Vec::new();
    for m in told.iter().filter(|m| m.value != "default") {
        if out.iter().any(|o: &ToldModel| o.id == m.value) {
            continue;
        }
        let is_default = !marked && default == Some(m.resolved_model.as_str());
        marked |= is_default;
        let efforts = if m.supports_effort == Some(true) {
            m.supported_effort_levels.clone()
        } else {
            Vec::new()
        };
        out.push(ToldModel {
            from_config: false,
            id: m.value.clone(),
            label: if m.display_name.is_empty() {
                m.value.clone()
            } else {
                m.display_name.clone()
            },
            efforts,
            default_effort: None,
            images: true,
            default: is_default,
            provider: None,
            multiplier: None,
            price: None,
        });
    }
    if !marked {
        if let Some(first) = out.first_mut() {
            first.default = true;
        }
    }
    out
}
