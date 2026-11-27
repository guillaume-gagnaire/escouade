//! « Mettre à jour tout seuls » (ESC-700): which offered updates install without a gesture. Exactly those the window
//! for all of « Tout mettre à jour » would agree to together: a version of the store, verified here (its seal held with
//! the app's keys, the revocation list read first), that asks for nothing more than the version installed (no
//! permission, no harness line, no contribution). Everything else stays offered and waits for a gesture. A plugin whose
//! harness has an agent in its turn waits for that turn's end; an error leaves the version installed, is said, and that
//! version is not tried again until the app starts again.

use crate::i18n::Lang;
use crate::manifest::Manifest;
use crate::{described, sort, store, Origin, PluginView, Plugins, Preview, Refusal};
use escouade_model::model::PluginsAutoUpdate;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

/// Why an offered update does not install by itself: it stays offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Held {
    /// « Jamais »: the setting is off.
    Off,
    /// « Les plugins d'Escouade » and a plugin of a third party (not `dev.escouade.*`).
    ThirdParty,
    /// Not a plugin of the store: a git repository, a folder, a folder in development, the workspace's.
    NotStore,
    /// The version the app's seed holds: installed from there, by a gesture.
    Seed,
    /// The version offered is not verified (or its seal was lost).
    NotVerified,
    /// The revocation list withdraws or unverifies the version offered.
    Revoked,
    /// The version would be loaded switched off (its API, its range, a `run.command`).
    Refused,
    /// No manifest installed to compare with.
    Unknown,
    /// It changes the line of a harness it describes (program, arguments, variables).
    HarnessLine,
    /// A permission the version installed did not have.
    AsksMore,
    /// A contribution the version installed did not have (or one gone).
    Contributions,
}

/// Whether the offered update of `p` may be read to install by itself, before anything is downloaded.
pub fn offered(
    scope: PluginsAutoUpdate,
    p: &PluginView,
    list: Option<&store::RevocationList>,
) -> Result<(), Held> {
    let Some(u) = &p.update else {
        return Err(Held::Off);
    };
    match scope {
        PluginsAutoUpdate::Never => return Err(Held::Off),
        PluginsAutoUpdate::Escouade if !crate::builtin::first_hand(&p.id) => {
            return Err(Held::ThirdParty)
        }
        _ => {}
    }
    if !matches!(p.origin, Origin::Store { .. }) {
        return Err(Held::NotStore);
    }
    if u.seed {
        return Err(Held::Seed);
    }
    if u.verified != Some(true) {
        return Err(Held::NotVerified);
    }
    if list.is_some_and(|l| {
        l.find(&p.id, &u.version, "disable").is_some()
            || l.find(&p.id, &u.version, "unverify").is_some()
    }) {
        return Err(Held::Revoked);
    }
    Ok(())
}

/// Whether the version read (`preview`) asks for nothing more than the one installed (`installed`): what the window for
/// all of « Tout mettre à jour » holds (`asksMore` of src/lib/plugins-view.svelte.ts, ESC-438).
pub fn agreed(preview: &Preview, installed: &PluginView) -> Result<(), Held> {
    if !preview.verified || preview.unverified.is_some() {
        return Err(Held::NotVerified);
    }
    if !matches!(preview.origin, Origin::Store { seed: false, .. }) {
        return Err(Held::NotStore);
    }
    if preview.refusal.is_some() {
        return Err(Held::Refused);
    }
    let Some(before) = &installed.manifest else {
        return Err(Held::Unknown);
    };
    if preview.previous.is_none() {
        return Err(Held::Unknown);
    }
    if preview.grown.iter().any(|w| w.starts_with("harness")) {
        return Err(Held::HarnessLine);
    }
    if !preview.grown.is_empty() {
        return Err(Held::AsksMore);
    }
    if !same_contributions(&preview.manifest, before) {
        return Err(Held::Contributions);
    }
    Ok(())
}

/// The points of a manifest whose items are compared by their names (`contributionsOf` of src/lib/plugins.ts).
const POINTS: [&str; 10] = [
    "themes",
    "harnesses",
    "ticketSources",
    "forges",
    "launchEnvironments",
    "accounts",
    "webhooks",
    "commands",
    "settings",
    "secrets",
];

/// Whether `next` contributes what `before` did, point by point, by the names of their items: their values may change
/// (a theme's colours), not what they are.
pub fn same_contributions(next: &Manifest, before: &Manifest) -> bool {
    let names = |m: &Manifest, key: &str| -> Vec<String> {
        let mut out: Vec<String> = m
            .contributes
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|i| {
                let pick = ["id", "name", "title", "events"]
                    .iter()
                    .filter_map(|k| i.get(*k).map(|v| format!("{k}={v}")))
                    .collect::<Vec<_>>();
                pick.join(";")
            })
            .collect();
        out.sort();
        out
    };
    POINTS.iter().all(|k| names(next, k) == names(before, k))
}

/// Whether the plugin `p` serves a harness of `busy` (the harnesses an agent is in a turn on): its own plugin
/// (`sort::of_engine`), or a harness its manifest describes.
pub fn serves(p: &PluginView, busy: &[String]) -> bool {
    busy.iter().any(|engine| {
        sort::of_engine(engine).contains(&p.id.as_str())
            || p.manifest.as_ref().is_some_and(|m| {
                described::harnesses_of(&m.contributes)
                    .iter()
                    .any(|h| &h.id == engine)
            })
    })
}

/// An update made by itself, as the page Mises à jour and the tab Versions say it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdated {
    pub from: String,
    pub to: String,
    /// When (RFC 3339).
    pub at: String,
}

/// An update by itself that failed: the version installed stays, said on the page Mises à jour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoFailed {
    pub id: String,
    pub name: String,
    pub version: String,
    pub message: String,
    pub at: String,
}

/// What a pass of `auto_update` did.
#[derive(Debug, Default)]
pub struct AutoReport {
    /// Installed by itself: (the plugin as it is now, the version it had).
    pub installed: Vec<(PluginView, String)>,
    /// Kept for the end of a turn: their harness has an agent in its turn.
    pub waiting: Vec<String>,
    pub failed: Vec<AutoFailed>,
    /// Left offered, and why.
    pub held: Vec<(String, Held)>,
}

impl Plugins {
    /// Installs by itself every update `offered` and `agreed` (ESC-700). `busy` says whether a plugin serves a harness
    /// an agent is in its turn on (it waits); `tried` holds the versions already tried by itself since the app started,
    /// never tried again (an error does not loop); `install` installs what an inspection set aside, as a gesture would
    /// (the core's stops the plugin first and starts it again). The caller reads the store and its revocation list first.
    #[allow(clippy::too_many_arguments)]
    pub async fn auto_update(
        &self,
        store: &store::Store,
        http: &reqwest::Client,
        scope: PluginsAutoUpdate,
        busy: &(dyn Fn(&PluginView) -> bool + Sync),
        tried: &parking_lot::Mutex<HashSet<(String, String)>>,
        install: &(dyn Fn(&str) -> Result<PluginView, Refusal> + Sync),
        lang: Lang,
    ) -> AutoReport {
        let mut report = AutoReport::default();
        if scope == PluginsAutoUpdate::Never {
            return report;
        }
        let list = store.revocations();
        for p in self.list(lang) {
            let Some(u) = p.update.clone() else { continue };
            if let Err(why) = offered(scope, &p, list.as_ref()) {
                report.held.push((p.id.clone(), why));
                continue;
            }
            if tried.lock().contains(&(p.id.clone(), u.version.clone())) {
                continue;
            }
            if busy(&p) {
                report.waiting.push(p.id.clone());
                continue;
            }
            tried.lock().insert((p.id.clone(), u.version.clone()));
            let failed = |message: String| AutoFailed {
                id: p.id.clone(),
                name: p.name.clone(),
                version: u.version.clone(),
                message,
                at: chrono::Utc::now().to_rfc3339(),
            };
            // Read as « N'installer que des versions vérifiées » reads it: no seal, no version.
            let fetched = match store.fetch(http, &p.id, Some(&u.version), true, lang).await {
                Ok(f) => f,
                Err(e) => {
                    report.failed.push(failed(e));
                    continue;
                }
            };
            let preview = match self.inspect_store(fetched, lang) {
                Ok(v) => v,
                Err(r) => {
                    report.failed.push(failed(r.message));
                    continue;
                }
            };
            if let Err(why) = agreed(&preview, &p) {
                self.discard(&preview.token);
                report.held.push((p.id.clone(), why));
                continue;
            }
            // A turn began while it was read: it waits for its end, not tried yet.
            if busy(&p) {
                self.discard(&preview.token);
                tried.lock().remove(&(p.id.clone(), u.version.clone()));
                report.waiting.push(p.id.clone());
                continue;
            }
            match install(&preview.token) {
                Ok(view) => {
                    let mark = AutoUpdated {
                        from: p.version.clone(),
                        to: view.version.clone(),
                        at: chrono::Utc::now().to_rfc3339(),
                    };
                    if let Err(e) = self.choose(&view.id, |c| c.auto_updated = Some(mark)) {
                        log::warn!("plugin {}: its update by itself is not noted: {e}", view.id);
                    }
                    let view = self.get(&view.id, lang).unwrap_or(view);
                    report.installed.push((view, p.version.clone()));
                }
                Err(r) => report.failed.push(failed(r.message)),
            }
        }
        report
    }
}
