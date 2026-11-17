//! The extensions of the editor's view (ESC-481, A5 of docs/EDITOR-STUDY.md), by the core: `escouade_vsx` with the
//! app's client (its proxy, its authorities), its data folder (`<data>/extensions`) and the layers of the allowed list.
//! The page asks everything here: the search, the cards, their icons and READMEs, the installs and updates, the files
//! of an installed extension (raw, with their type). It never reaches Open VSX itself.
//!
//! The layers of `extensions.allowed`, the strongest first, each of which must allow an extension: the managed device's
//! and the team's (the locks, TS-11), this device's settings, the `.escouade.yml` of each project open in the view.
//! A private project asks nothing of Open VSX (no search, no card, no install, no update): the least said.
//!
//! The protocol `escouade-ext:` serves the same files (`escouade_vsx::serve`), read-only. The window's CSP does not
//! name it: the page reads the files by `vsx_file`; the protocol is for the extension host of its own origin (A6).

use crate::core::Core;
use escouade_vsx::allowed::{Layer, Rules};
use escouade_vsx::install::{State, Update};
use escouade_vsx::registry::{Detail, Page, Registry};
use escouade_vsx::store::Store;
use escouade_vsx::unpack::Limits;
use escouade_vsx::{Code, Refusal};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// The extensions of this app: their store, the registry asked.
pub struct Extensions {
    pub store: Store,
    base: String,
    limits: Limits,
}

impl Extensions {
    pub fn new(data: &Path) -> Arc<Extensions> {
        Arc::new(Extensions::at(data, &registry_base()))
    }

    /// The store of `data`, asking the registry at `base`.
    pub fn at(data: &Path, base: &str) -> Extensions {
        Extensions {
            store: Store::open(&data.join("extensions")),
            base: base.to_owned(),
            limits: Limits::default(),
        }
    }

    /// The registry, asked with the app's client: its proxy, its authorities; a VSIX may take a while.
    pub fn registry(&self, settings: &crate::model::Settings) -> Result<Registry, Refusal> {
        let client = crate::tls::app_builder(settings)
            .and_then(|b| {
                b.connect_timeout(Duration::from_secs(15))
                    .timeout(Duration::from_secs(300))
                    .build()
            })
            .map_err(Refusal::network)?;
        Registry::new(&self.base, client)
    }
}

/// Open VSX; in a build of development, `ESCOUADE_TEST_OPEN_VSX` names a fake one (the e2e's). Never in a release.
fn registry_base() -> String {
    #[cfg(debug_assertions)]
    if let Some(url) = std::env::var("ESCOUADE_TEST_OPEN_VSX")
        .ok()
        .filter(|u| !u.is_empty())
    {
        return url;
    }
    escouade_vsx::registry::OPEN_VSX.to_owned()
}

/// What the view's projects make of the allowed list, and whether one of them is private.
#[derive(Clone, Debug, Default)]
pub struct Context {
    pub layers: Vec<Layer>,
    pub private: Option<String>,
    pub warnings: Vec<String>,
}

/// The layers of the allowed list for the projects `projects` (those open in the view): the imposed ones (each
/// project's locks, the device's alone when none), the device's settings, each project's file.
pub fn context(core: &Core, projects: &[String]) -> Context {
    let mut out = Context::default();
    let mut imposed: Vec<crate::locks::Imposed> = Vec::new();
    let locks: Vec<crate::locks::Locks> = if projects.is_empty() {
        vec![core.locks.read().clone()]
    } else {
        projects.iter().map(|p| core.locks_for(p)).collect()
    };
    for l in locks {
        for i in l.extension_layers() {
            if !imposed.contains(&i) {
                imposed.push(i);
            }
        }
    }
    let push = |by: String, imposed: bool, v: &serde_json::Value, out: &mut Context| {
        if v.is_null() {
            return;
        }
        let (rules, warnings) = Rules::parse(v);
        out.warnings.extend(warnings);
        out.layers.push(Layer { by, imposed, rules });
    };
    for i in imposed {
        push(i.by.by(), true, &i.value, &mut out);
    }
    let device = core.settings.read().extensions_allowed.clone();
    push(
        tr!("par les réglages de ce poste", "by this device’s settings"),
        false,
        &device,
        &mut out,
    );
    for id in projects {
        let Ok(p) = core.project(id) else { continue };
        if p.private && out.private.is_none() {
            out.private = Some(p.name.clone());
        }
        let file = p.shared.file.as_ref().and_then(|f| {
            f.values
                .get(crate::projectfile::EXTENSIONS_ALLOWED)
                .cloned()
        });
        if let Some(v) = file {
            let name = &p.name;
            push(
                tr!(
                    "par le .escouade.yml de {name}",
                    "by the .escouade.yml of {name}"
                ),
                false,
                &v,
                &mut out,
            );
        }
    }
    out
}

/// A private project's refusal: nothing is asked of Open VSX from it.
pub fn private_refusal(name: &str) -> Refusal {
    Refusal::new(
        Code::Private,
        tr!(
            "« {name} » est un projet privé : Escouade ne demande rien à Open VSX depuis lui (ni recherche, ni fiche, ni mise à jour). Les extensions déjà installées y marchent.",
            "“{name}” is a private project: Escouade asks nothing of Open VSX from it (no search, no card, no update). The extensions already installed work there."
        ),
    )
}

/// A result, or the refusal the page says.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome<T> {
    pub value: Option<T>,
    pub refusal: Option<Refusal>,
}

impl<T> Outcome<T> {
    pub fn of(r: Result<T, Refusal>) -> Self {
        match r {
            Ok(v) => Outcome {
                value: Some(v),
                refusal: None,
            },
            Err(e) => Outcome {
                value: None,
                refusal: Some(e),
            },
        }
    }
}

/// A card, with what the page shows beside it: its README (a third party's Markdown, rendered sanitized by the
/// page), what the allowed list says of it, the version installed.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub detail: Detail,
    pub readme: Option<String>,
    pub refused: Option<String>,
    pub installed: Option<String>,
}

/// What is installed, as the view takes it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub extensions: Vec<State>,
    pub theme: Option<String>,
    pub layers: Vec<Layer>,
    pub warnings: Vec<String>,
    /// The private project of the view, if one is: nothing is asked of Open VSX.
    pub private: Option<String>,
}

pub fn installed(ext: &Extensions, ctx: &Context) -> Installed {
    Installed {
        extensions: ext
            .store
            .list()
            .into_iter()
            .map(|e| escouade_vsx::install::state(e, &ctx.layers))
            .collect(),
        theme: ext.store.theme(),
        layers: ctx.layers.clone(),
        warnings: ctx.warnings.clone(),
        private: ctx.private.clone(),
    }
}

pub async fn search(
    ext: &Extensions,
    core: &Core,
    ctx: &Context,
    query: &str,
    offset: u32,
) -> Result<Page, Refusal> {
    if let Some(p) = &ctx.private {
        return Err(private_refusal(p));
    }
    let registry = ext.registry(&core.settings.read())?;
    registry.search(query, offset, 20).await
}

pub async fn card(
    ext: &Extensions,
    core: &Core,
    ctx: &Context,
    id: &str,
    version: Option<&str>,
) -> Result<Card, Refusal> {
    if let Some(p) = &ctx.private {
        return Err(private_refusal(p));
    }
    let ext_id = escouade_vsx::ExtId::parse(id)?;
    let registry = ext.registry(&core.settings.read())?;
    let detail = registry.detail(&ext_id, version).await?;
    let readme = match detail.files.readme.as_deref() {
        Some(url) => registry
            .bytes(url, 512 << 10)
            .await
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned()),
        None => None,
    };
    let refused = escouade_vsx::allowed::check(
        &ctx.layers,
        &ext_id,
        Some((&detail.version, detail.pre_release)),
    )
    .err()
    .map(|r| r.message);
    Ok(Card {
        installed: ext.store.get(id).map(|e| e.version),
        detail,
        readme,
        refused,
    })
}

/// An icon or a capture of a card, asked by the core when the page shows it: a data URL of an image (PNG, JPEG, GIF,
/// WebP, SVG), at most 1 MB, from the registry alone.
pub async fn image(
    ext: &Extensions,
    core: &Core,
    ctx: &Context,
    url: &str,
) -> Result<String, Refusal> {
    use base64::Engine;
    if let Some(p) = &ctx.private {
        return Err(private_refusal(p));
    }
    let registry = ext.registry(&core.settings.read())?;
    let bytes = registry.bytes(url, 1 << 20).await?;
    let mime = image_type(&bytes).ok_or_else(|| Refusal::new(Code::NotFound, "image"))?;
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// The type of an image, by its first bytes (never by what the server says).
pub fn image_type(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if b.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        let head = String::from_utf8_lossy(&b[..b.len().min(512)]).to_lowercase();
        (head.trim_start().starts_with("<svg") || head.contains("<svg")).then_some("image/svg+xml")
    }
}

pub async fn install(
    ext: &Extensions,
    core: &Core,
    ctx: &Context,
    id: &str,
    version: Option<&str>,
    trust: bool,
) -> Result<State, Refusal> {
    if let Some(p) = &ctx.private {
        return Err(private_refusal(p));
    }
    let registry = ext.registry(&core.settings.read())?;
    let e = escouade_vsx::install::install(
        &registry,
        &ext.store,
        &ext.limits,
        id,
        version,
        &ctx.layers,
        trust,
    )
    .await?;
    Ok(escouade_vsx::install::state(e, &ctx.layers))
}

pub async fn updates(ext: &Extensions, core: &Core, ctx: &Context) -> Result<Vec<Update>, Refusal> {
    if let Some(p) = &ctx.private {
        return Err(private_refusal(p));
    }
    let registry = ext.registry(&core.settings.read())?;
    Ok(escouade_vsx::install::updates(&registry, &ext.store, &ctx.layers).await)
}

/// A file of an installed extension, as the page registers it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    pub mime: &'static str,
    pub size: u64,
}

/// The files of an installed extension (no link, nothing outside its folder), at most 2000.
pub fn files(ext: &Extensions, id: &str) -> Result<Vec<FileInfo>, Refusal> {
    let Some(dir) = ext.store.dir_of(id) else {
        return Err(Refusal::new(Code::NotFound, id.to_owned()));
    };
    let mut out = Vec::new();
    let mut stack = vec![(dir.clone(), String::new())];
    while let Some((at, rel)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for e in entries.flatten() {
            let Ok(kind) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().into_owned();
            let path = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            if kind.is_dir() {
                stack.push((e.path(), path));
            } else if kind.is_file() {
                let size = e.metadata().map_or(0, |m| m.len());
                out.push(FileInfo {
                    mime: escouade_vsx::serve::mime_of(&path),
                    path,
                    size,
                });
                if out.len() >= 2000 {
                    return Ok(out);
                }
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// The protocol's answer (`escouade-ext:`), read-only.
pub fn protocol(
    ext: Option<&Extensions>,
    request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    match ext {
        Some(ext) => escouade_vsx::serve::respond(
            &ext.store,
            request.method().as_str(),
            request.uri().path(),
        ),
        None => tauri::http::Response::builder()
            .status(503)
            .body(Vec::new())
            .expect("a fixed response"),
    }
}

#[cfg(test)]
#[path = "vsx_tests.rs"]
mod tests;

// ---------- the commands of the window ----------

type Core_<'a> = crate::remoteapi::State<'a, Arc<Core>>;
type Ext<'a> = crate::remoteapi::State<'a, Arc<Extensions>>;

/// What is installed, and the layers of the allowed list for the view's projects (no request to Open VSX).
#[escouade_command::command]
pub fn vsx_installed(core: Core_<'_>, ext: Ext<'_>, projects: Vec<String>) -> Installed {
    installed(&ext, &context(&core, &projects))
}

/// A page of the search (20 results from `offset`).
#[escouade_command::command]
pub async fn vsx_search(
    core: Core_<'_>,
    ext: Ext<'_>,
    projects: Vec<String>,
    query: String,
    offset: u32,
) -> Result<Outcome<Page>, String> {
    let ctx = context(&core, &projects);
    Ok(Outcome::of(search(&ext, &core, &ctx, &query, offset).await))
}

/// The card of `id` (at `version`, its latest when none), with its README.
#[escouade_command::command]
pub async fn vsx_card(
    core: Core_<'_>,
    ext: Ext<'_>,
    projects: Vec<String>,
    id: String,
    version: Option<String>,
) -> Result<Outcome<Card>, String> {
    let ctx = context(&core, &projects);
    Ok(Outcome::of(
        card(&ext, &core, &ctx, &id, version.as_deref()).await,
    ))
}

/// An icon or a capture of a card, as a data URL.
#[escouade_command::command]
pub async fn vsx_image(
    core: Core_<'_>,
    ext: Ext<'_>,
    projects: Vec<String>,
    url: String,
) -> Result<Outcome<String>, String> {
    let ctx = context(&core, &projects);
    Ok(Outcome::of(image(&ext, &core, &ctx, &url).await))
}

/// Installs `id` (or updates it, to `version`, or to its latest). `trust`: the user trusted its publisher.
#[escouade_command::command]
pub async fn vsx_install(
    core: Core_<'_>,
    ext: Ext<'_>,
    projects: Vec<String>,
    id: String,
    version: Option<String>,
    trust: bool,
) -> Result<Outcome<State>, String> {
    let ctx = context(&core, &projects);
    Ok(Outcome::of(
        install(&ext, &core, &ctx, &id, version.as_deref(), trust).await,
    ))
}

/// The updates of what is installed: proposed, never installed by themselves.
#[escouade_command::command]
pub async fn vsx_updates(
    core: Core_<'_>,
    ext: Ext<'_>,
    projects: Vec<String>,
) -> Result<Outcome<Vec<Update>>, String> {
    let ctx = context(&core, &projects);
    Ok(Outcome::of(updates(&ext, &core, &ctx).await))
}

#[escouade_command::command]
pub fn vsx_set_enabled(ext: Ext<'_>, id: String, enabled: bool) -> Result<(), String> {
    ext.store
        .set_enabled(&id, enabled)
        .map(|_| ())
        .map_err(|r| r.message)
}

#[escouade_command::command]
pub fn vsx_remove(ext: Ext<'_>, id: String) -> Result<(), String> {
    ext.store.remove(&id).map_err(|r| r.message)
}

/// The colour theme of an installed extension chosen in the view (`<id>/<label>`), none: Escouade's again.
#[escouade_command::command]
pub fn vsx_set_theme(ext: Ext<'_>, theme: Option<String>) -> Result<(), String> {
    ext.store
        .set_theme(theme.map(|t| t.to_lowercase_id()))
        .map_err(|r| r.message)
}

/// The files of an installed extension.
#[escouade_command::command]
pub fn vsx_files(ext: Ext<'_>, id: String) -> Result<Vec<FileInfo>, String> {
    files(&ext, &id).map_err(|r| r.message)
}

/// A file of an installed extension, raw (`escouade_vsx::serve::read`: inside its folder alone).
#[escouade_command::command]
pub fn vsx_file(ext: Ext<'_>, id: String, path: String) -> Result<tauri::ipc::Response, String> {
    escouade_vsx::serve::read(&ext.store, &id, &path)
        .map(|(bytes, _)| tauri::ipc::Response::new(bytes))
        .map_err(|r| r.message)
}

/// `<id>/<label>` with its id in lower case, as the store keeps it.
trait LowerId {
    fn to_lowercase_id(self) -> String;
}

impl LowerId for String {
    fn to_lowercase_id(self) -> String {
        match self.split_once('/') {
            Some((id, label)) => format!("{}/{label}", id.to_lowercase()),
            None => self,
        }
    }
}
