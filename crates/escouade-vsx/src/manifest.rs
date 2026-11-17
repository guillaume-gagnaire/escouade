//! What an extension is, from its `package.json`: declarative (themes, grammars, snippets, languages, icons: it runs in
//! the page without any code of its), with code for the web host (`browser`: it waits for the extension host of its own
//! origin, A6), or for Node (`main` alone: it asks for the Node host, which Escouade has not yet).

use crate::{Code, ExtId, Refusal, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Declarative,
    /// Code for the web extension host (`browser`).
    Web,
    /// Code for Node (`main`, no `browser`).
    Node,
}

/// What it brings, as the page shows it and registers it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributes {
    /// The ids of its colour themes, as the setting `workbench.colorTheme` names them (`id`, else `label`).
    pub themes: Vec<String>,
    pub icon_themes: Vec<String>,
    /// The languages it names (ids).
    pub languages: Vec<String>,
    pub grammars: usize,
    pub snippets: usize,
}

/// An extension's manifest, as read.
#[derive(Clone, Debug, PartialEq)]
pub struct Manifest {
    pub publisher: String,
    pub name: String,
    pub version: String,
    pub kind: Kind,
    pub contributes: Contributes,
    pub raw: Value,
}

fn labels(v: &Value, key: &str) -> Vec<String> {
    v.get("contributes")
        .and_then(|c| c.get(key))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    t.get("id")
                        .or_else(|| t.get("label"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn count(v: &Value, key: &str) -> usize {
    v.get("contributes")
        .and_then(|c| c.get(key))
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// The kind of an extension from its manifest.
pub fn kind_of(v: &Value) -> Kind {
    if text(v, "browser").is_some() {
        Kind::Web
    } else if text(v, "main").is_some() {
        Kind::Node
    } else {
        Kind::Declarative
    }
}

/// `package.json`, which must be the extension `id` at `version` (the card's): another one is refused.
pub fn read(bytes: &[u8], id: &ExtId, version: &str) -> Result<Manifest> {
    let mismatch = |why: String| {
        Refusal::new(
            Code::Mismatch,
            tr!(
                "L’archive n’est pas l’extension de la fiche : {why}.",
                "The archive is not the card’s extension: {why}."
            ),
        )
    };
    let raw: Value = serde_json::from_slice(bytes).map_err(|e| {
        mismatch(tr!(
            "son package.json ne se lit pas ({e})",
            "its package.json cannot be read ({e})"
        ))
    })?;
    let publisher = text(&raw, "publisher").unwrap_or_default();
    let name = text(&raw, "name").unwrap_or_default();
    let v = text(&raw, "version").unwrap_or_default();
    let got = format!("{publisher}.{name}");
    if !got.eq_ignore_ascii_case(&id.to_string()) {
        return Err(mismatch(tr!("elle est « {got} »", "it is “{got}”")));
    }
    if v != version {
        return Err(mismatch(tr!(
            "elle est en version {v}",
            "it is version {v}"
        )));
    }
    let languages = raw
        .get("contributes")
        .and_then(|c| c.get("languages"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|l| l.get("id").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Ok(Manifest {
        publisher,
        name,
        version: v,
        kind: kind_of(&raw),
        contributes: Contributes {
            themes: labels(&raw, "themes"),
            icon_themes: labels(&raw, "iconThemes"),
            languages,
            grammars: count(&raw, "grammars"),
            snippets: count(&raw, "snippets"),
        },
        raw,
    })
}
