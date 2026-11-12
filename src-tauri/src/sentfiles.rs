//! The files an agent sends to the user (ESC-171): Claude Code's `SendUserFile` tool, whose input
//! names them (`files`, with a `caption`, a `status`, a `display`). Its images are copied, when the
//! call comes, into the agent's folder of the data folder (`attachments/<agent>/sent-<call>/`),
//! for they often live in a temporary folder of its worktree that goes with it: the conversation
//! shows them still. The copies go with the agent, or with the history (`attach_keep_days`, as the
//! files joined to messages). Its other files stay where they are, by their path.
//!
//! The window never reads a path: it asks for the files of a call by its agent and its item, and
//! the core reads them, recognising an image by its bytes, never by its extension, under a size
//! (`MAX_IMAGE`, `MAX_CALL`), and never a file the agent's fence refuses (a private project's,
//! ESC-140). A file gone since is told as such.

use crate::attachfiles::safe_name;
use crate::fence::Fence;
use crate::projectlogo::data_url;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};

/// The name of Claude Code's tool.
pub(crate) const TOOL: &str = "SendUserFile";

/// The heaviest image shown (and copied): a screenshot or a render, even in 4K, weighs less.
pub(crate) const MAX_IMAGE: u64 = 10 << 20;

/// The most a call's images may weigh together, copied: those after are shown from where they are.
pub(crate) const MAX_CALL: u64 = 50 << 20;

/// What the bytes are, of the images shown: PNG, JPEG, GIF, WebP, SVG (as an image, its scripts
/// never run).
pub(crate) fn image_kind(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    let head = &b[..b.len().min(4096)];
    let text = String::from_utf8_lossy(head.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(head));
    let text = text.trim_start();
    (text.starts_with('<') && text.to_ascii_lowercase().contains("<svg")).then_some("image/svg+xml")
}

/// The path of a file the agent names, on this machine: as written (`C:/…` with its `/` under
/// Windows), under its folder `cwd` when relative, through the share of its distribution when its
/// folder is in WSL and the path is Linux's.
pub(crate) fn resolve(cwd: &str, file: &str) -> PathBuf {
    let file = file.trim();
    if cfg!(windows) && file.starts_with('/') && !file.starts_with("//") {
        if let Some((distro, _)) = crate::wsl::share_path(&cwd.replace('\\', "/")) {
            return PathBuf::from(crate::wsl::to_windows(file, distro, "/mnt/"));
        }
    }
    let path = Path::new(file);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(cwd).join(path)
    }
}

/// The name a file is known by: the last part of its path.
fn name_of(file: &str) -> String {
    file.trim()
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|n| !n.is_empty())
        .unwrap_or(file)
        .to_string()
}

/// The paths of the files a call names.
pub(crate) fn files_of(input: &Value) -> Vec<String> {
    input["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f.as_str())
        .filter(|f| !f.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// How many of the files a call names are images, by their extension (its card says it before
/// they are read).
pub(crate) fn count_images(input: &Value) -> (usize, usize) {
    let files = files_of(input);
    let images = files
        .iter()
        .filter(|f| {
            let lower = f.to_ascii_lowercase();
            [".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg"]
                .iter()
                .any(|e| lower.ends_with(e))
        })
        .count();
    (images, files.len() - images)
}

/// The first bytes of a file, enough to know its format.
fn head_of(path: &Path) -> Option<Vec<u8>> {
    let mut head = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(4096)
        .read_to_end(&mut head)
        .ok()?;
    Some(head)
}

/// The files of the call `call` of an agent, as its item keeps them (`sent`): each image copied
/// under `folder` (the agent's, `DataDir::agent_files`) while the call's copies weigh `MAX_CALL`
/// at most, the other files by their path. A file the fence refuses keeps its name only; one not
/// there is said missing; an image heavier than `MAX_IMAGE` is a file.
pub(crate) fn deliver(folder: &Path, call: &str, cwd: &str, input: &Value, fence: &Fence) -> Value {
    let dir = folder.join(format!("sent-{}", safe_name(call)));
    let mut copied = 0u64;
    let entries: Vec<Value> = files_of(input)
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let name = name_of(file);
            let path = resolve(cwd, file);
            let shown = path.to_string_lossy().to_string();
            if fence.covers(&shown) {
                return json!({ "name": name, "kind": "file", "state": "fenced" });
            }
            let file = format!("{i}-{}", safe_name(&name));
            let Some(meta) = std::fs::metadata(&path).ok().filter(|m| m.is_file()) else {
                // Delivered already (its frames read again), its file gone since: its copy.
                let copy = dir.join(&file);
                let media = head_of(&copy).as_deref().and_then(image_kind);
                return match (media, std::fs::metadata(&copy)) {
                    (Some(media), Ok(m)) => json!({
                        "name": name, "path": shown, "size": m.len(), "state": "ok", "kind": "image",
                        "mediaType": media, "copy": format!("{}/{file}", dir.file_name().unwrap_or_default().to_string_lossy()),
                    }),
                    _ => json!({ "name": name, "path": shown, "kind": "file", "state": "missing" }),
                };
            };
            let size = meta.len();
            let image = head_of(&path).as_deref().and_then(image_kind);
            let mut entry = json!({ "name": name, "path": shown, "size": size, "state": "ok" });
            match image {
                Some(media) if size <= MAX_IMAGE => {
                    entry["kind"] = json!("image");
                    entry["mediaType"] = json!(media);
                    if copied + size <= MAX_CALL {
                        let made = std::fs::create_dir_all(&dir)
                            .and_then(|_| std::fs::copy(&path, dir.join(&file)));
                        match made {
                            Ok(_) => {
                                copied += size;
                                entry["copy"] = json!(format!(
                                    "{}/{file}",
                                    dir.file_name().unwrap_or_default().to_string_lossy()
                                ));
                            }
                            Err(e) => log::warn!("sent image {shown} not copied: {e}"),
                        }
                    }
                }
                Some(_) => {
                    entry["kind"] = json!("file");
                    entry["big"] = json!(true);
                }
                None => entry["kind"] = json!("file"),
            }
            entry
        })
        .collect();
    Value::Array(entries)
}

/// What the window shows of a file sent: its name, image or file, its size, whether it is there
/// (`ok`), gone (`missing`), refused (`fenced`), and an image's `data:` URL.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SentView {
    pub name: String,
    pub kind: &'static str,
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// An image too heavy to be shown: a file.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub big: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// A program or a script (`runnable`): never opened, only shown in its folder.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub runnable: bool,
}

/// An image's `data:` URL, read from `path` when it is one of the images shown and no heavier than
/// `MAX_IMAGE`.
fn image_url(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_IMAGE {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let media = image_kind(&bytes)?;
    Some(data_url(media, &bytes))
}

/// The copy `copy` of `folder`: a name the item keeps (`sent-<call>/<file>`), never one that
/// leads out of it.
fn copy_path(folder: &Path, copy: &str) -> Option<PathBuf> {
    let (dir, file) = copy.split_once('/')?;
    let plain = |s: &str| !s.is_empty() && s != "." && s != ".." && !s.contains(['/', '\\', ':']);
    (dir.starts_with("sent-") && plain(dir) && plain(file)).then(|| folder.join(dir).join(file))
}

/// The files of a call as the window shows them now. `sent` is what its item keeps (`deliver`);
/// a call that has none (shown before it was delivered, or of a conversation older than this) is
/// read from its `input`, the files where they are.
pub(crate) fn view(
    folder: &Path,
    cwd: &str,
    input: &Value,
    sent: Option<&Value>,
    fence: &Fence,
) -> Vec<SentView> {
    let live;
    let entries = match sent.and_then(Value::as_array) {
        Some(entries) => entries,
        None => {
            live = files_of(input)
                .iter()
                .map(|file| {
                    let path = resolve(cwd, file).to_string_lossy().to_string();
                    json!({ "name": name_of(file), "path": path, "kind": "pending" })
                })
                .collect::<Vec<_>>();
            &live
        }
    };
    entries
        .iter()
        .map(|e| {
            let name = e["name"].as_str().unwrap_or_default().to_string();
            let path = e["path"].as_str().map(PathBuf::from);
            let runs = runnable_name(&name) || path.as_deref().is_some_and(runnable);
            let refused = SentView {
                name: name.clone(),
                kind: "file",
                state: "fenced",
                size: None,
                big: false,
                url: None,
                runnable: runs,
            };
            if e["state"] == "fenced"
                || path
                    .as_ref()
                    .is_some_and(|p| fence.covers(&p.to_string_lossy()))
            {
                return refused;
            }
            let big = e["big"] == true;
            if e["kind"] == "image" || e["kind"] == "pending" {
                // The copy first, else the file where it is.
                let from_copy = e["copy"]
                    .as_str()
                    .and_then(|c| copy_path(folder, c))
                    .and_then(|p| image_url(&p));
                let url = from_copy.or_else(|| {
                    (e["copy"].is_null())
                        .then(|| path.as_deref().and_then(image_url))
                        .flatten()
                });
                if let Some(url) = url {
                    return SentView {
                        name,
                        kind: "image",
                        state: "ok",
                        size: e["size"].as_u64(),
                        big: false,
                        url: Some(url),
                        runnable: runs,
                    };
                }
            }
            // A file, or an image no longer there: what its path holds now.
            let meta = path
                .as_deref()
                .and_then(|p| std::fs::metadata(p).ok())
                .filter(|m| m.is_file());
            let image = e["kind"] == "image";
            SentView {
                name,
                kind: if image { "image" } else { "file" },
                state: if meta.is_some() && !image {
                    "ok"
                } else {
                    "missing"
                },
                size: meta.as_ref().map(|m| m.len()),
                big: big
                    || (e["kind"] == "pending"
                        && meta.is_some()
                        && path
                            .as_deref()
                            .and_then(head_of)
                            .as_deref()
                            .and_then(image_kind)
                            .is_some()),
                url: None,
                runnable: runs,
            }
        })
        .collect()
}

/// The path of the file `index` of a call, to open or show in its folder: refused when the fence
/// refuses it, or when it is no longer there. An image is its copy when it has one.
#[allow(clippy::too_many_arguments)]
pub(crate) fn target(
    folder: &Path,
    cwd: &str,
    input: &Value,
    sent: Option<&Value>,
    index: usize,
    reveal: bool,
    fence: &Fence,
    lang: crate::i18n::Lang,
) -> anyhow::Result<PathBuf> {
    let entry = match sent.and_then(Value::as_array) {
        Some(entries) => entries.get(index).cloned(),
        None => files_of(input)
            .get(index)
            .map(|f| json!({ "name": name_of(f), "path": resolve(cwd, f).to_string_lossy() })),
    };
    let Some(entry) = entry else {
        anyhow::bail!("no file {index}");
    };
    let path = entry["path"].as_str().map(PathBuf::from);
    if entry["state"] == "fenced"
        || path
            .as_ref()
            .is_some_and(|p| fence.covers(&p.to_string_lossy()))
    {
        anyhow::bail!(tr_in!(
            lang,
            "Ce fichier est d’un projet privé",
            "This file belongs to a private project"
        ));
    }
    let name = entry["name"].as_str().unwrap_or_default();
    let copy = entry["copy"].as_str().and_then(|c| copy_path(folder, c));
    let Some(found) = [path, copy].into_iter().flatten().find(|p| p.is_file()) else {
        anyhow::bail!(tr_in!(
            lang,
            "Fichier introuvable : {name}",
            "File not found: {name}"
        ));
    };
    // A program the agent sent is never run, whatever the window asks: shown in its folder only.
    if !reveal && (runnable_name(name) || runnable(&found)) {
        return Err(not_run(lang, name));
    }
    Ok(found)
}

/// The refusal to open a program or a script `name`: Escouade only shows it in its folder.
fn not_run(lang: crate::i18n::Lang, name: &str) -> anyhow::Error {
    anyhow::anyhow!(tr_in!(
        lang,
        "« {name} » est un programme ou un script : Escouade ne l’ouvre pas, elle l’affiche dans son dossier.",
        "“{name}” is a program or a script: Escouade doesn’t open it, it shows it in its folder."
    ))
}

/// `path` opened with its application, or shown in its folder (`reveal`).
fn open_or_reveal(path: PathBuf, reveal: bool) -> anyhow::Result<()> {
    if reveal {
        crate::fsedit::reveal(crate::fsedit::Reveal::Select(path))
    } else {
        crate::outside::open(crate::outside::What::Path(&path))
    }
}

/// The extensions of what the system runs when it opens it: programs, scripts, installers,
/// shortcuts, of Windows, macOS and Linux alike (a file of one is sent to any of them).
const RUNNABLE: &[&str] = &[
    "exe",
    "com",
    "bat",
    "cmd",
    "ps1",
    "vbs",
    "vbe",
    "js",
    "jse",
    "wsf",
    "wsh",
    "msi",
    "msp",
    "scr",
    "pif",
    "lnk",
    "url",
    "hta",
    "cpl",
    "reg",
    "jar",
    "appref-ms",
    "app",
    "command",
    "sh",
    "pkg",
    "dmg",
    "desktop",
    "appimage",
];

/// Whether the name `name` is one of a program or a script (`RUNNABLE`), whatever its case.
pub(crate) fn runnable_name(name: &str) -> bool {
    let name = name.trim().trim_end_matches(['.', ' ', '/', '\\']);
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| RUNNABLE.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Whether the file `path` is a program or a script: by its name, or, under macOS and Linux,
/// marked executable.
pub(crate) fn runnable(path: &Path) -> bool {
    if runnable_name(&path.to_string_lossy()) {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        {
            return true;
        }
    }
    false
}

/// The image a message of the agent shows in Markdown by a local path (`![…](C:/…/a.png)`,
/// `file:///…`): its `data:` URL, read as a file sent is, its fence kept. None for what is no
/// image of the formats shown, heavier than `MAX_IMAGE`, gone, or refused.
pub(crate) fn local_image(cwd: &str, src: &str, fence: &Fence) -> Option<String> {
    image_url(&local_path(cwd, src, fence)?)
}

/// The file a message names by a local path (`C:/…/a.png`, `file:///…`, `%20` decoded), on this
/// machine: None when the fence refuses it.
fn local_path(cwd: &str, src: &str, fence: &Fence) -> Option<PathBuf> {
    let src = src.trim();
    let src = src
        .strip_prefix("file:///")
        .or_else(|| src.strip_prefix("file://"))
        .unwrap_or(src);
    let path = resolve(cwd, &percent_decode(src));
    (!fence.covers(&path.to_string_lossy())).then_some(path)
}

/// The image a message shows by a local path, to open or show in its folder (the image viewer):
/// only an image it shows (`local_image`), never a program opened.
pub(crate) fn local_target(
    cwd: &str,
    src: &str,
    reveal: bool,
    fence: &Fence,
    lang: crate::i18n::Lang,
) -> anyhow::Result<PathBuf> {
    let Some(path) = local_path(cwd, src, fence).filter(|p| image_url(p).is_some()) else {
        anyhow::bail!(tr_in!(
            lang,
            "Image introuvable : {src}",
            "Image not found: {src}"
        ));
    };
    if !reveal && runnable(&path) {
        return Err(not_run(lang, &name_of(src)));
    }
    Ok(path)
}

/// The bytes of an image the window shows (a `data:` URL the core made) and the name to save it
/// under (« Enregistrer sous… » of the image viewer): refused when they are no image of the formats
/// shown. The name made safe (`safe_name`), its format's extension added when it has none.
pub(crate) fn image_to_save(data_url: &str, name: &str) -> anyhow::Result<(Vec<u8>, String)> {
    use base64::Engine;
    let bytes = data_url
        .strip_prefix("data:image/")
        .and_then(|rest| rest.split_once(";base64,"))
        .and_then(|(_, b)| base64::engine::general_purpose::STANDARD.decode(b).ok())
        .unwrap_or_default();
    let Some(media) = image_kind(&bytes) else {
        anyhow::bail!("not an image");
    };
    let mut name = safe_name(name);
    if Path::new(&name).extension().is_none() {
        name.push('.');
        name.push_str(match media {
            "image/jpeg" => "jpg",
            "image/svg+xml" => "svg",
            m => m.trim_start_matches("image/"),
        });
    }
    Ok((bytes, name))
}

/// `%20` and the like of a path written as a URL, as bytes; the rest as it is.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The heaviest image of the web downloaded, as a local one.
pub(crate) const MAX_REMOTE: u64 = MAX_IMAGE;

/// The address of an image of the web a message shows, checked: https, with a host, without a
/// name or a password in it. Anything else is refused before any request.
pub(crate) fn remote_url(lang: crate::i18n::Lang, src: &str) -> anyhow::Result<reqwest::Url> {
    let refused = || {
        anyhow::anyhow!(tr_in!(
            lang,
            "Seule une image en https s’affiche",
            "Only an https image is shown"
        ))
    };
    let url = reqwest::Url::parse(src.trim()).map_err(|_| refused())?;
    let plain = url.scheme() == "https"
        && url.host_str().is_some_and(|h| !h.is_empty())
        && url.username().is_empty()
        && url.password().is_none();
    if !plain {
        return Err(refused());
    }
    Ok(url)
}

fn remote_too_big(lang: crate::i18n::Lang) -> anyhow::Error {
    let max = crate::core::size_label(lang, MAX_REMOTE as usize);
    anyhow::anyhow!(tr_in!(
        lang,
        "L’image pèse plus de {max}",
        "The image weighs more than {max}"
    ))
}

/// `chunk` added to what was downloaded, `buf`: refused past `MAX_REMOTE`, the download stops.
pub(crate) fn take_chunk(
    lang: crate::i18n::Lang,
    buf: &mut Vec<u8>,
    chunk: &[u8],
) -> anyhow::Result<()> {
    if (buf.len() + chunk.len()) as u64 > MAX_REMOTE {
        return Err(remote_too_big(lang));
    }
    buf.extend_from_slice(chunk);
    Ok(())
}

/// What was downloaded as a `data:` URL, when its bytes are an image of the formats shown.
pub(crate) fn remote_data_url(lang: crate::i18n::Lang, bytes: &[u8]) -> anyhow::Result<String> {
    let Some(media) = image_kind(bytes) else {
        anyhow::bail!(tr_in!(
            lang,
            "Ce n’est pas une image PNG, JPEG, GIF, WebP ou SVG",
            "It isn’t a PNG, JPEG, GIF, WebP or SVG image"
        ));
    };
    Ok(data_url(media, bytes))
}

/// The image of the web `src`, downloaded by the core on the user's click (never by the window,
/// which would load any address an agent wrote, a secret in it): https only, redirects too,
/// `MAX_REMOTE` at most, recognised by its bytes; through the proxy of the settings, without a
/// cookie, a referrer or a user agent that would tell who asks.
pub(crate) async fn fetch_image(
    settings: &crate::model::Settings,
    lang: crate::i18n::Lang,
    src: &str,
) -> anyhow::Result<String> {
    let url = remote_url(lang, src)?;
    let b = crate::tls::app_builder(settings)?
        .timeout(std::time::Duration::from_secs(20))
        .referer(false)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.url().scheme() != "https" {
                attempt.error("redirected out of https")
            } else if attempt.previous().len() >= 5 {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }));
    let mut res = b.build()?.get(url).send().await?.error_for_status()?;
    if res.content_length().is_some_and(|l| l > MAX_REMOTE) {
        return Err(remote_too_big(lang));
    }
    let mut buf = Vec::new();
    while let Some(chunk) = res.chunk().await? {
        take_chunk(lang, &mut buf, &chunk)?;
    }
    remote_data_url(lang, &buf)
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// An image of the web a message shows, downloaded at the user's click (`fetch_image`).
    pub async fn remote_image(&self, src: &str) -> anyhow::Result<String> {
        let settings = self.settings.read().clone();
        fetch_image(&settings, crate::i18n::ui(), src).await
    }

    /// The files of the agent `id`'s call `item` copied and said on its item (`deliver`), off
    /// the async workers. A call delivered already (its conversation read again) is left as it is.
    pub(crate) fn deliver_sent(self: &std::sync::Arc<Self>, id: &str, item: String, input: Value) {
        let Ok(agent) = self.agent(id) else {
            return;
        };
        let cwd = {
            let mut rt = agent.lock();
            if rt.conv.get(&item).is_none_or(|v| v.get("sent").is_some()) {
                return;
            }
            rt.meta.cwd.clone()
        };
        let (folder, fence) = (self.data.agent_files(id), self.agent_fence(id));
        let (core, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            let call = item.clone();
            let sent =
                tokio::task::spawn_blocking(move || deliver(&folder, &call, &cwd, &input, &fence))
                    .await;
            let Ok(sent) = sent else {
                return;
            };
            let mut fx = crate::agent::Effects::default();
            let (pid, name) = {
                let mut rt = agent.lock();
                if rt.conv.get(&item).is_none() {
                    return;
                }
                rt.patch(&item, json!({ "sent": sent }), &mut fx);
                fx.save = true;
                (rt.meta.project_id.clone(), rt.meta.name.clone())
            };
            core.apply(&id, &pid, &name, fx, None);
        });
    }

    /// What the window needs of the call `item` of the agent `id`: its folder of copies, its
    /// working folder, the call's input, what its item keeps of its files, the agent's fence.
    fn sent_call(
        &self,
        id: &str,
        item: &str,
    ) -> anyhow::Result<(PathBuf, String, Value, Option<Value>, Fence)> {
        let agent = self.agent(id)?;
        let (cwd, input, sent) = {
            let mut rt = agent.lock();
            let cwd = rt.meta.cwd.clone();
            let Some(v) = rt
                .conv
                .get(item)
                .filter(|v| v["kind"] == "tool" && v["name"] == TOOL)
            else {
                anyhow::bail!("no call {item}");
            };
            (cwd, v["input"].clone(), v.get("sent").cloned())
        };
        Ok((
            self.data.agent_files(id),
            cwd,
            input,
            sent,
            self.agent_fence(id),
        ))
    }

    /// The files of a call as the window shows them now (`view`).
    pub async fn sent_files(&self, id: &str, item: &str) -> anyhow::Result<Vec<SentView>> {
        let (folder, cwd, input, sent, fence) = self.sent_call(id, item)?;
        Ok(
            tokio::task::spawn_blocking(move || view(&folder, &cwd, &input, sent.as_ref(), &fence))
                .await?,
        )
    }

    /// The file `index` of a call opened with its application, or shown in its folder (`reveal`).
    pub async fn open_sent_file(
        &self,
        id: &str,
        item: &str,
        index: usize,
        reveal: bool,
    ) -> anyhow::Result<()> {
        let (folder, cwd, input, sent, fence) = self.sent_call(id, item)?;
        let lang = crate::i18n::ui();
        tokio::task::spawn_blocking(move || {
            let path = target(
                &folder,
                &cwd,
                &input,
                sent.as_ref(),
                index,
                reveal,
                &fence,
                lang,
            )?;
            open_or_reveal(path, reveal)
        })
        .await?
    }

    /// An image a message of the agent `id` shows by a local path (`local_image`).
    pub async fn agent_local_image(&self, id: &str, src: &str) -> anyhow::Result<Option<String>> {
        let cwd = self.agent(id)?.lock().meta.cwd.clone();
        let (fence, src) = (self.agent_fence(id), src.to_string());
        Ok(tokio::task::spawn_blocking(move || local_image(&cwd, &src, &fence)).await?)
    }

    /// That image opened with its application, or shown in its folder (`reveal`): `local_target`.
    pub async fn open_local_image(&self, id: &str, src: &str, reveal: bool) -> anyhow::Result<()> {
        let cwd = self.agent(id)?.lock().meta.cwd.clone();
        let (fence, src, lang) = (self.agent_fence(id), src.to_string(), crate::i18n::ui());
        tokio::task::spawn_blocking(move || {
            open_or_reveal(local_target(&cwd, &src, reveal, &fence, lang)?, reveal)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    const GIF: &[u8] = b"GIF89a\x01\0\x01\0";
    const PDF: &[u8] = b"%PDF-1.7\n%\xE2\xE3";

    fn file(dir: &Path, name: &str, bytes: &[u8]) -> String {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        path.to_string_lossy().replace('\\', "/")
    }

    #[test]
    fn the_five_formats_are_known_by_their_bytes_not_their_extension() {
        assert_eq!(image_kind(PNG), Some("image/png"));
        assert_eq!(image_kind(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(image_kind(GIF), Some("image/gif"));
        assert_eq!(image_kind(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(
            image_kind(b"\xEF\xBB\xBF<?xml?>\n<svg xmlns='x'/>"),
            Some("image/svg+xml")
        );
        assert_eq!(image_kind(PDF), None);
        assert_eq!(image_kind(b"<html><body>no image</body></html>"), None);
        assert_eq!(image_kind(b""), None);
    }

    #[test]
    fn images_are_copied_and_outlive_their_files_the_others_stay_by_their_path() {
        let root = test_dir("sent-deliver");
        let (work, folder) = (root.join("wt"), root.join("data").join("agent"));
        // Paths as Claude Code writes them under Windows, with `/`; one relative to its folder.
        let a = file(&work, ".impeccable/tmp/a.png", PNG);
        let b = file(&work, "b.gif", GIF);
        let pdf = file(&work, "contract.pdf", PDF);
        // An image by its extension, a text by its bytes: a file.
        let fake = file(&work, "fake.png", b"not an image");
        let input = json!({ "files": [a, "b.gif", pdf, fake, "gone.png"], "caption": "c" });
        let cwd = work.to_string_lossy().to_string();
        let sent = deliver(&folder, "toolu_01", &cwd, &input, &Fence::default());
        let kinds: Vec<_> = sent
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["name"].as_str().unwrap(),
                    e["kind"].as_str().unwrap(),
                    e["state"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("a.png", "image", "ok"),
                ("b.gif", "image", "ok"),
                ("contract.pdf", "file", "ok"),
                ("fake.png", "file", "ok"),
                ("gone.png", "file", "missing"),
            ]
        );
        assert_eq!(sent[0]["copy"], "sent-toolu_01/0-a.png");
        assert_eq!(sent[2]["size"], PDF.len());
        assert!(sent[2].get("copy").is_none());
        let _ = b;
        // The worktree goes: the images show still, the other files are said gone.
        std::fs::remove_dir_all(&work).unwrap();
        let shown = view(&folder, &cwd, &input, Some(&sent), &Fence::default());
        assert_eq!(shown[0].kind, "image");
        assert_eq!(shown[0].state, "ok");
        assert!(shown[0]
            .url
            .as_deref()
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(shown[1]
            .url
            .as_deref()
            .unwrap()
            .starts_with("data:image/gif;base64,"));
        assert_eq!((shown[2].kind, shown[2].state), ("file", "missing"));
        assert_eq!((shown[4].kind, shown[4].state), ("file", "missing"));
        // Its copy gone too (the agent's folder erased), an image is said gone.
        std::fs::remove_dir_all(&folder).unwrap();
        let shown = view(&folder, &cwd, &input, Some(&sent), &Fence::default());
        assert_eq!(
            (shown[0].kind, shown[0].state, shown[0].url.is_none()),
            ("image", "missing", true)
        );
        let e = target(
            &folder,
            &cwd,
            &input,
            Some(&sent),
            2,
            false,
            &Fence::default(),
            crate::i18n::Lang::Fr,
        )
        .unwrap_err();
        assert_eq!(e.to_string(), "Fichier introuvable : contract.pdf");
    }

    #[test]
    fn an_image_too_heavy_is_a_file_and_a_call_copies_up_to_its_maximum() {
        let root = test_dir("sent-heavy");
        let (work, folder) = (root.join("wt"), root.join("agent"));
        let mut heavy = PNG.to_vec();
        heavy.resize(MAX_IMAGE as usize + 1, 0);
        let big = file(&work, "big.png", &heavy);
        let input = json!({ "files": [big] });
        let cwd = work.to_string_lossy().to_string();
        let sent = deliver(&folder, "t2", &cwd, &input, &Fence::default());
        assert_eq!(
            (sent[0]["kind"].as_str(), sent[0]["big"].as_bool()),
            (Some("file"), Some(true))
        );
        assert!(!folder.join("sent-t2").exists());
        let shown = view(&folder, &cwd, &input, Some(&sent), &Fence::default());
        assert_eq!(
            (shown[0].kind, shown[0].state, shown[0].big),
            ("file", "ok", true)
        );
        assert!(shown[0].url.is_none());
        // Images of 4 MB, 13 of them: the twelfth would pass 50 MB, it and the next are not copied
        // and show from where they are.
        let mut four = PNG.to_vec();
        four.resize(4 << 20, 0);
        let files: Vec<String> = (0..13)
            .map(|i| file(&work, &format!("p{i}.png"), &four))
            .collect();
        let input = json!({ "files": files });
        let sent = deliver(&folder, "t3", &cwd, &input, &Fence::default());
        let copied = sent
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e.get("copy").is_some())
            .count();
        assert_eq!(copied, 12);
        assert!(sent[12].get("copy").is_none());
        let shown = view(&folder, &cwd, &input, Some(&sent), &Fence::default());
        assert!(shown.iter().all(|s| s.kind == "image" && s.url.is_some()));
    }

    #[test]
    fn the_viewer_opens_only_an_image_a_message_shows_and_saves_only_an_image() {
        let root = test_dir("sent-viewer");
        let (mine, private) = (root.join("mine"), root.join("private"));
        let secret = file(&private, "secret.png", PNG);
        file(&mine, "a b.png", PNG);
        file(&mine, "notes.pdf", PDF);
        let (fence, cwd) = (
            Fence::new(vec![private.clone()]),
            mine.to_string_lossy().to_string(),
        );
        let open = |src: &str| local_target(&cwd, src, false, &fence, crate::i18n::Lang::En);
        assert_eq!(open("a%20b.png").unwrap(), mine.join("a b.png"));
        assert!(open("notes.pdf").is_err());
        assert!(open(secret.as_str()).is_err());
        assert!(open("gone.png").is_err());

        let url = data_url("image/png", PNG);
        assert_eq!(
            image_to_save(&url, "plan").unwrap(),
            (PNG.to_vec(), "plan.png".into())
        );
        assert_eq!(image_to_save(&url, "a/b.png").unwrap().1, "a_b.png");
        assert!(image_to_save(&data_url("image/png", PDF), "x.png").is_err());
        assert!(image_to_save(&data_url("application/pdf", PNG), "x").is_err());
        assert!(image_to_save("https://example.com/a.png", "a.png").is_err());
    }

    #[test]
    fn a_file_the_fence_refuses_is_never_read_copied_nor_opened() {
        let root = test_dir("sent-fence");
        let (mine, private, folder) = (root.join("mine"), root.join("private"), root.join("agent"));
        let secret = file(&private, "secret.png", PNG);
        let ok = file(&mine, "ok.png", PNG);
        let fence = Fence::new(vec![private.clone()]);
        let input = json!({ "files": [secret, ok] });
        let cwd = mine.to_string_lossy().to_string();
        let sent = deliver(&folder, "t4", &cwd, &input, &fence);
        assert_eq!(
            sent[0],
            json!({ "name": "secret.png", "kind": "file", "state": "fenced" })
        );
        assert_eq!(sent[1]["kind"], "image");
        let shown = view(&folder, &cwd, &input, Some(&sent), &fence);
        assert_eq!((shown[0].state, shown[0].url.is_none()), ("fenced", true));
        assert!(target(
            &folder,
            &cwd,
            &input,
            Some(&sent),
            0,
            false,
            &fence,
            crate::i18n::Lang::En
        )
        .is_err());
        // A call read from its input alone (an older conversation): the same.
        let shown = view(&folder, &cwd, &input, None, &fence);
        assert_eq!((shown[0].state, shown[0].url.is_none()), ("fenced", true));
        assert_eq!((shown[1].kind, shown[1].state), ("image", "ok"));
        let e = target(
            &folder,
            &cwd,
            &input,
            None,
            0,
            false,
            &fence,
            crate::i18n::Lang::En,
        )
        .unwrap_err();
        assert_eq!(e.to_string(), "This file belongs to a private project");
    }

    #[test]
    fn a_call_without_its_copies_is_read_where_its_files_are() {
        let root = test_dir("sent-live");
        let work = root.join("wt");
        let png = file(&work, "a.png", PNG);
        let pdf = file(&work, "b.pdf", PDF);
        let input = json!({ "files": [png, pdf, "nowhere.txt"] });
        let cwd = work.to_string_lossy().to_string();
        let shown = view(&root.join("agent"), &cwd, &input, None, &Fence::default());
        assert_eq!((shown[0].kind, shown[0].state), ("image", "ok"));
        assert_eq!(
            (shown[1].kind, shown[1].state, shown[1].size),
            ("file", "ok", Some(PDF.len() as u64))
        );
        assert_eq!((shown[2].kind, shown[2].state), ("file", "missing"));
        let path = target(
            &root,
            &cwd,
            &input,
            None,
            1,
            false,
            &Fence::default(),
            crate::i18n::Lang::Fr,
        )
        .unwrap();
        assert_eq!(path, work.join("b.pdf"));
    }

    #[test]
    fn a_program_or_a_script_is_never_opened_only_shown_in_its_folder() {
        for name in [
            "setup.EXE",
            "a.cmd",
            "a.bat",
            "run.ps1",
            "x.vbs",
            "x.js",
            "x.msi",
            "x.lnk",
            "x.url",
            "x.hta",
            "x.reg",
            "x.jar",
            "x.appref-ms",
            "Tool.app",
            "go.command",
            "go.sh",
            "x.pkg",
            "x.dmg",
            "x.desktop",
            "x.AppImage",
            "x.scr",
            "x.cpl",
        ] {
            assert!(runnable_name(name), "{name}");
        }
        for name in [
            "a.pdf",
            "a.png",
            "notes.txt",
            "page.html",
            "a.zip",
            "exe",
            "json.json",
        ] {
            assert!(!runnable_name(name), "{name}");
        }
        let root = test_dir("sent-runnable");
        let work = root.join("wt");
        let script = file(&work, "deploy.ps1", b"Remove-Item -Recurse C:/");
        let pdf = file(&work, "doc.pdf", PDF);
        let cwd = work.to_string_lossy().to_string();
        let input = json!({ "files": [script, pdf] });
        let sent = deliver(&root.join("agent"), "t9", &cwd, &input, &Fence::default());
        let shown = view(
            &root.join("agent"),
            &cwd,
            &input,
            Some(&sent),
            &Fence::default(),
        );
        assert!(shown[0].runnable && !shown[1].runnable);
        let lang = crate::i18n::Lang::Fr;
        // Opened: refused by the core, whatever the window asks; shown in its folder: yes.
        let e = target(
            &root,
            &cwd,
            &input,
            Some(&sent),
            0,
            false,
            &Fence::default(),
            lang,
        )
        .unwrap_err();
        assert!(e.to_string().contains("programme ou un script"), "{e}");
        assert!(target(
            &root,
            &cwd,
            &input,
            Some(&sent),
            0,
            true,
            &Fence::default(),
            lang
        )
        .is_ok());
        assert!(target(
            &root,
            &cwd,
            &input,
            Some(&sent),
            1,
            false,
            &Fence::default(),
            lang
        )
        .is_ok());
        // A program whose name says nothing, marked executable (macOS, Linux).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let bin = work.join("tool");
            std::fs::write(&bin, b"\x7fELF").unwrap();
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(runnable(&bin));
        }
    }

    #[test]
    fn an_image_of_the_web_is_https_an_image_and_not_too_heavy() {
        let lang = crate::i18n::Lang::Fr;
        assert!(remote_url(lang, "https://example.com/a.png?x=1").is_ok());
        for bad in [
            "http://example.com/a.png",
            "ftp://example.com/a.png",
            "file:///C:/a.png",
            "https://user:pw@example.com/a.png",
            "data:image/png;base64,AA==",
            "nothing",
        ] {
            let e = remote_url(lang, bad).unwrap_err();
            assert_eq!(e.to_string(), "Seule une image en https s’affiche", "{bad}");
        }
        // Its bytes, not its address, say it is an image.
        assert!(remote_data_url(lang, PNG)
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(remote_data_url(lang, PDF).is_err());
        assert!(remote_data_url(lang, b"<html>no</html>").is_err());
        // Too heavy: the download stops at the chunk that passes the maximum.
        let mut buf = vec![0; MAX_REMOTE as usize - 2];
        assert!(take_chunk(lang, &mut buf, b"ab").is_ok());
        let e = take_chunk(lang, &mut buf, b"c").unwrap_err();
        assert_eq!(e.to_string(), "L’image pèse plus de 10 Mo");
        assert_eq!(buf.len() as u64, MAX_REMOTE);
    }

    #[tokio::test]
    async fn an_image_of_the_web_not_in_https_is_refused_before_any_request() {
        let settings = crate::model::Settings::default();
        let e = fetch_image(&settings, crate::i18n::Lang::En, "http://127.0.0.1:9/a.png")
            .await
            .unwrap_err();
        assert_eq!(e.to_string(), "Only an https image is shown");
    }

    #[test]
    fn a_copy_named_by_hand_never_leads_out_of_the_agents_folder() {
        let folder = Path::new("/data/agent");
        assert!(copy_path(folder, "sent-t/0-a.png").is_some());
        for bad in [
            "../x/a.png",
            "sent-t/../../a.png",
            "sent-t/..",
            "other/a.png",
            "sent-t/C:a",
            "sent-t",
        ] {
            assert!(copy_path(folder, bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn the_cards_count_says_images_and_files() {
        let input = json!({ "files": ["a.PNG", "b.jpeg", "c.pdf"] });
        assert_eq!(count_images(&input), (2, 1));
        assert_eq!(count_images(&json!({})), (0, 0));
    }

    #[test]
    fn a_relative_path_is_under_the_agents_folder() {
        let cwd = if cfg!(windows) { "C:/w" } else { "/w" };
        assert_eq!(resolve(cwd, "x/a.png"), Path::new(cwd).join("x/a.png"));
        let abs = if cfg!(windows) {
            "D:/z/a.png"
        } else {
            "/z/a.png"
        };
        assert_eq!(resolve(cwd, abs), PathBuf::from(abs));
        #[cfg(windows)]
        assert_eq!(
            resolve(r"\\wsl.localhost\Ubuntu\home\me\p", "/home/me/p/a.png"),
            PathBuf::from(r"\\wsl.localhost\Ubuntu\home\me\p\a.png")
        );
    }
}
