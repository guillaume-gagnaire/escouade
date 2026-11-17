//! Open VSX's API, by the core: `/api/-/search`, `/api/<namespace>/<name>[/<version>]` and the files of a version,
//! with the client the app gives (through its proxy, trusting its authorities). What Open VSX answers is a third
//! party's data: every address it hands is checked to be on the registry's own origin before it is asked (its storage
//! may answer after a redirection), every body is read within a bound, every name and version checked.

use crate::{is_name, is_version, Code, ExtId, Refusal, Result};
use serde::Serialize;
use serde_json::Value;

/// Open VSX.
pub const OPEN_VSX: &str = "https://open-vsx.org";

/// A result of a search.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    /// Whether Open VSX verified its namespace (its owner is who publishes there).
    pub verified: bool,
    pub icon: Option<String>,
    pub downloads: u64,
    pub deprecated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub extensions: Vec<Summary>,
    pub total: u64,
}

/// The files of a version, as the card names them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Files {
    pub download: Option<String>,
    pub sha256: Option<String>,
    pub signature: Option<String>,
    pub public_key: Option<String>,
    pub readme: Option<String>,
    pub icon: Option<String>,
    pub manifest: Option<String>,
}

/// A card of a version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub id: String,
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub verified: bool,
    /// Its icon's address on the registry (the core downloads it when the page shows it).
    pub icon: Option<String>,
    /// Who published it, as Open VSX shows them.
    pub publisher: String,
    pub license: String,
    pub repository: String,
    pub homepage: String,
    pub pre_release: bool,
    pub deprecated: bool,
    pub downloadable: bool,
    pub downloads: u64,
    pub timestamp: String,
    /// Its other versions, the newest first.
    pub versions: Vec<String>,
    pub categories: Vec<String>,
    pub files: Files,
}

pub struct Registry {
    base: url::Url,
    client: reqwest::Client,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_control())
        .take(2000)
        .collect()
}

fn summary(v: &Value) -> Option<Summary> {
    let namespace = s(v, "namespace");
    let name = s(v, "name");
    let version = s(v, "version");
    if !is_name(&namespace) || !is_name(&name) || !is_version(&version) {
        return None;
    }
    let display = s(v, "displayName");
    Some(Summary {
        id: format!("{namespace}.{name}"),
        display_name: if display.is_empty() {
            name.clone()
        } else {
            display
        },
        description: s(v, "description"),
        verified: v.get("verified").and_then(Value::as_bool).unwrap_or(false),
        icon: v
            .pointer("/files/icon")
            .and_then(Value::as_str)
            .map(str::to_owned),
        downloads: v.get("downloadCount").and_then(Value::as_u64).unwrap_or(0),
        deprecated: v
            .get("deprecated")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        namespace,
        name,
        version,
    })
}

impl Registry {
    /// The registry at `base` (`https://open-vsx.org`, or a fake one in the tests), asked with `client`.
    pub fn new(base: &str, client: reqwest::Client) -> Result<Registry> {
        let base = url::Url::parse(base).map_err(Refusal::network)?;
        if !matches!(base.scheme(), "https" | "http") {
            return Err(Refusal::network("https"));
        }
        Ok(Registry { base, client })
    }

    pub fn base(&self) -> &url::Url {
        &self.base
    }

    /// Whether `url` is on the registry's origin: the only addresses asked.
    pub fn owns(&self, url: &str) -> bool {
        url::Url::parse(url).is_ok_and(|u| u.origin() == self.base.origin())
    }

    /// `url` asked, its body read up to `max` bytes; a status other than 2xx is `NotFound` (404) or `Network`.
    pub async fn bytes(&self, url: &str, max: u64) -> Result<Vec<u8>> {
        if !self.owns(url) {
            return Err(Refusal::new(
                Code::Network,
                tr!(
                    "Open VSX a donné une adresse hors de chez lui : elle n’est pas demandée.",
                    "Open VSX gave an address outside of it: it is not asked."
                ),
            ));
        }
        let mut resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(Refusal::network)?;
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Refusal::new(
                Code::NotFound,
                tr!("Open VSX ne la connaît pas.", "Open VSX does not know it."),
            ));
        }
        if !status.is_success() {
            return Err(Refusal::network(status));
        }
        if resp.content_length().is_some_and(|n| n > max) {
            return Err(too_big(max));
        }
        let mut out = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(Refusal::network)? {
            out.extend_from_slice(&chunk);
            if out.len() as u64 > max {
                return Err(too_big(max));
            }
        }
        Ok(out)
    }

    async fn json(&self, url: &str) -> Result<Value> {
        let bytes = self.bytes(url, 4 << 20).await?;
        serde_json::from_slice(&bytes).map_err(Refusal::network)
    }

    fn api(&self, path: &str) -> String {
        let mut u = self.base.clone();
        u.set_path(path);
        u.to_string()
    }

    /// A page of results for `query`.
    pub async fn search(&self, query: &str, offset: u32, size: u32) -> Result<Page> {
        let mut u = url::Url::parse(&self.api("/api/-/search")).map_err(Refusal::network)?;
        u.query_pairs_mut()
            .append_pair("query", query.trim())
            .append_pair("offset", &offset.to_string())
            .append_pair("size", &size.clamp(1, 50).to_string())
            .append_pair("includeAllVersions", "false");
        let v = self.json(u.as_str()).await?;
        Ok(Page {
            extensions: v
                .get("extensions")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(summary).collect())
                .unwrap_or_default(),
            total: v.get("totalSize").and_then(Value::as_u64).unwrap_or(0),
        })
    }

    /// The card of `id`, at `version` (its latest when none).
    pub async fn detail(&self, id: &ExtId, version: Option<&str>) -> Result<Detail> {
        let path = match version {
            Some(v) if is_version(v) => format!("/api/{}/{}/{v}", id.namespace, id.name),
            Some(v) => return Err(Refusal::new(Code::BadId, v.to_owned())),
            None => format!("/api/{}/{}", id.namespace, id.name),
        };
        let v = self.json(&self.api(&path)).await?;
        if v.get("error").is_some() {
            return Err(Refusal::new(
                Code::NotFound,
                tr!(
                    "Open VSX ne connaît pas « {id} ».",
                    "Open VSX does not know “{id}”."
                ),
            ));
        }
        let sum = summary(&v).ok_or_else(|| Refusal::new(Code::Mismatch, id.to_string()))?;
        if !sum.id.eq_ignore_ascii_case(&id.to_string()) {
            return Err(Refusal::new(Code::Mismatch, sum.id));
        }
        let file = |k: &str| {
            v.pointer(&format!("/files/{k}"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        let mut versions: Vec<String> = v
            .get("allVersions")
            .and_then(Value::as_object)
            .map(|m| m.keys().filter(|k| is_version(k)).cloned().collect())
            .unwrap_or_default();
        versions.sort_by(|a, b| compare_versions(b, a));
        let by = v.get("publishedBy").cloned().unwrap_or(Value::Null);
        let publisher = [
            s(&by, "fullName"),
            s(&by, "loginName"),
            s(&v, "namespaceDisplayName"),
        ]
        .into_iter()
        .find(|x| !x.is_empty())
        .unwrap_or_else(|| sum.namespace.clone());
        Ok(Detail {
            publisher,
            license: s(&v, "license"),
            repository: s(&v, "repository"),
            homepage: s(&v, "homepage"),
            pre_release: v
                .get("preRelease")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            downloadable: v
                .get("downloadable")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            timestamp: s(&v, "timestamp"),
            versions,
            categories: v
                .get("categories")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            files: Files {
                download: file("download"),
                sha256: file("sha256"),
                signature: file("signature"),
                public_key: file("publicKey"),
                readme: file("readme"),
                icon: file("icon"),
                manifest: file("manifest"),
            },
            id: sum.id,
            namespace: sum.namespace,
            name: sum.name,
            version: sum.version,
            display_name: sum.display_name,
            description: sum.description,
            verified: sum.verified,
            icon: sum.icon,
            deprecated: sum.deprecated,
            downloads: sum.downloads,
        })
    }
}

fn too_big(max: u64) -> Refusal {
    let mb = max >> 20;
    Refusal::new(
        Code::Trapped,
        tr!(
            "Le fichier dépasse {mb} Mo : il n’est pas téléchargé.",
            "The file is over {mb} MB: it is not downloaded."
        ),
    )
}

/// Two versions compared as numbers, part by part (`1.10.0` after `1.9.2`); a pre-release (`-beta`) before its release.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let split = |v: &str| {
        let (core, pre) = v.split_once('-').unwrap_or((v, ""));
        let core: Vec<u64> = core
            .split('+')
            .next()
            .unwrap_or_default()
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect();
        (core, pre.to_owned())
    };
    let (ca, pa) = split(a);
    let (cb, pb) = split(b);
    ca.cmp(&cb)
        .then_with(|| match (pa.is_empty(), pb.is_empty()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => pa.cmp(&pb),
        })
}
