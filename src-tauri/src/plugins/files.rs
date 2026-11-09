//! The files a plugin reaches through its host.
//!
//! - `files/read` (ESC-321, WASM-2): a file of the plugin's own folder, as text, for the SDK's `p.file(path)`, served the
//!   same to a plugin in WASM and to a process (schemas/plugin-protocol-1.json). Only inside the folder: a relative path,
//!   `/` between its parts, no `..`, nothing hidden (`.env`, `.git`), not the manifest; and, links followed, still inside
//!   the folder. 256 KiB at most, UTF-8.
//! - `fs/read`, `fs/write`, `fs/stat`, `fs/list`, `fs/canon` (ESC-333, docs/PLUGINS.md §8.4.6): what a plugin in WASM
//!   reads and writes of its agents' folders, by [`Access`]. It names a root of `permissions.files`, a closed list the
//!   core resolves, and never gets a folder of its own choice: the path, its links followed, must lie under a root of
//!   that very agent, out of the agent's fence (« Projet privé »), whatever the plugin's code says. Each write and each
//!   refusal goes to its journal.

use super::journal::Kind;
use super::manifest::{FileAccess, Manifest};
use base64::Engine as _;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// A path the plugin may not read: outside its folder, hidden, its manifest (as `net/fetch`'s refusals).
pub const REFUSED: i64 = -32003;
/// A file too big, or not text.
pub const FAILED: i64 = -32004;
/// No such file in its folder.
pub const NOT_FOUND: i64 = -32005;
/// The most a file may hold.
pub const FILE_BYTES: u64 = 256 << 10;
/// The most `fs/read` gives: its default, and its ceiling whatever `limits.fileBytes` says; `fs/write` takes as much.
pub const READ_BYTES: u64 = 256 << 10;
pub const MOST_BYTES: u64 = 4 << 20;
/// The most entries `fs/list` gives.
const LIST_ENTRIES: usize = 4096;

/// `{ path }` → the file's text, or why not.
pub fn read(folder: &Path, params: &Value) -> Result<Value, (i64, String)> {
    let Some(path) = params.get("path").and_then(Value::as_str) else {
        return Err((
            super::runtime::INVALID_PARAMS,
            "files/read takes a path".into(),
        ));
    };
    let parts = parts_of(path).ok_or_else(|| {
        (
            REFUSED,
            format!("{path:?} is not a file of the plugin's folder"),
        )
    })?;
    if parts.len() == 1 && parts[0] == super::manifest::FILE {
        return Err((
            REFUSED,
            format!("{path:?}: the manifest is not read by its plugin"),
        ));
    }
    let not_found = || {
        (
            NOT_FOUND,
            format!("no file {path:?} in the plugin's folder"),
        )
    };
    let root = std::fs::canonicalize(folder).map_err(|_| not_found())?;
    let target = parts.iter().fold(root.clone(), |p, s| p.join(s));
    let real = std::fs::canonicalize(&target).map_err(|_| not_found())?;
    // A link that leads out of the folder is outside it.
    if !real.starts_with(&root) {
        return Err((
            REFUSED,
            format!("{path:?} leads out of the plugin's folder"),
        ));
    }
    let meta = std::fs::metadata(&real).map_err(|_| not_found())?;
    if !meta.is_file() {
        return Err(not_found());
    }
    if meta.len() > FILE_BYTES {
        return Err((FAILED, format!("{path:?} is over 256 KiB")));
    }
    let bytes = std::fs::read(&real).map_err(|e| (FAILED, format!("{path:?}: {e}")))?;
    String::from_utf8(bytes)
        .map(Value::String)
        .map_err(|_| (FAILED, format!("{path:?} is not text (UTF-8)")))
}

/// Its parts, when it names a file of the folder: relative, `/` (or `\`) between them, none empty, `.`, `..` or
/// hidden, no drive nor colon.
fn parts_of(path: &str) -> Option<Vec<&str>> {
    if path.is_empty() || path.starts_with(['/', '\\']) || path.contains(['\0', ':']) {
        return None;
    }
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    parts
        .iter()
        .all(|p| !p.is_empty() && !p.starts_with('.'))
        .then_some(parts)
}

// ---- fs/* ------------------------------------------------------------------------------------------------------

/// How long `fs/gitProtected` and `fs/fileOnly` wait for git: well within the 2 s of a turn.
const GIT_WITHIN: std::time::Duration =
    std::time::Duration::from_secs(if cfg!(test) { 30 } else { 1 });

/// Where an agent lives, as the core resolved it: what `fs/*` and `proc/spawn` take for the agent named by a plugin. The
/// plugin is told its roots by `agent/attach`, and can change none of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentPlace {
    /// Its folder.
    pub cwd: PathBuf,
    /// The repository of its worktree; its folder when it has none.
    pub repo: Option<PathBuf>,
    /// The folder of its account.
    pub home: Option<PathBuf>,
    /// What « Projet privé » refuses it.
    pub fence: crate::fence::Fence,
    /// Its project.
    pub project: String,
}

impl AgentPlace {
    /// Its folder for the root `root`.
    fn root(&self, root: &str) -> Option<&Path> {
        match root {
            "agent" => Some(&self.cwd),
            "agentRepo" => self.repo.as_deref(),
            "agentHome" => self.home.as_deref(),
            _ => None,
        }
    }
}

/// The agents a plugin was given, by the key of `agent/attach`.
pub type Agents = Arc<Mutex<BTreeMap<String, AgentPlace>>>;

/// The repository `cwd` is a worktree of, or the one it is in: the folder that holds the real `.git`.
pub fn repo_of(cwd: &Path) -> Option<PathBuf> {
    let found = cwd.ancestors().find(|d| d.join(".git").exists())?;
    let dot = found.join(".git");
    if dot.is_dir() {
        return Some(found.to_path_buf());
    }
    // A worktree: `gitdir: <repository>/.git/worktrees/<name>`, absolute or relative to it.
    let text = std::fs::read_to_string(&dot).ok()?;
    let gitdir = found.join(text.strip_prefix("gitdir:")?.trim());
    let common = match std::fs::read_to_string(gitdir.join("commondir")) {
        Ok(c) => gitdir.join(c.trim()),
        Err(_) => gitdir,
    };
    std::fs::canonicalize(common)
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

/// How a file of a plugin is named, as `fs/*` take it.
struct Asked<'a> {
    /// The path as it was given, for the journal.
    path: &'a str,
    root: Option<&'a str>,
    agent: Option<&'a str>,
}

/// What a plugin may reach by `fs/*`.
pub struct Access {
    declared: Vec<FileAccess>,
    folder: PathBuf,
    temp: PathBuf,
    read_bytes: u64,
    agents: Agents,
}

impl Access {
    /// What `manifest` (its permissions as they are granted) reaches; `temp` is a folder of its own, made when it is
    /// first used.
    pub fn new(manifest: &Manifest, folder: &Path, temp: PathBuf, agents: Agents) -> Self {
        Self {
            declared: manifest.permissions.files.clone(),
            folder: folder.to_path_buf(),
            temp,
            read_bytes: manifest
                .limits
                .file_bytes
                .map_or(READ_BYTES, |n| n.min(MOST_BYTES)),
            agents,
        }
    }

    /// The roots `place` gives the plugin, what `agent/attach` tells it: those it declares, and that exist for this agent.
    pub fn roots_of(&self, place: &AgentPlace) -> BTreeMap<String, String> {
        self.declared
            .iter()
            .filter_map(|f| {
                let path = match f.root.as_str() {
                    "plugin" => self.folder.clone(),
                    "temp" => self.temp.clone(),
                    root => place.root(root)?.to_path_buf(),
                };
                Some((f.root.clone(), path.to_string_lossy().into_owned()))
            })
            .collect()
    }

    /// Serves `fs/<method>`. What it wrote, and any refusal, is said by `told`.
    pub fn serve(
        &self,
        method: &str,
        params: &Value,
        told: &dyn Fn(Kind, &str),
    ) -> Result<Value, (i64, String)> {
        let r = match method {
            "fs/read" => self.read(params),
            "fs/write" => self.write(params).inspect(|w| {
                told(
                    Kind::Message,
                    &format!("fs/write{}: wrote {} bytes", root_of(params), w["bytes"]),
                );
            }),
            "fs/stat" => self.stat(params),
            "fs/list" => self.list(params),
            "fs/canon" => self.canon(params),
            "fs/gitProtected" => self.git_protected(params),
            "fs/fileOnly" => self.file_only(params),
            _ => {
                return Err((
                    super::runtime::METHOD_NOT_FOUND,
                    format!("unknown method {method}"),
                ))
            }
        };
        r.inspect_err(|(code, message)| {
            if *code == REFUSED {
                // The path stays out of it, as a query stays out of `net/fetch`'s.
                let path = params.get("path").and_then(Value::as_str).unwrap_or("");
                let why = message.replace(&format!("{path:?}"), "the path");
                told(
                    Kind::Refused,
                    &format!("{method}{}: refused, {why}", root_of(params)),
                );
            }
        })
    }

    fn asked<'a>(&self, params: &'a Value) -> Result<Asked<'a>, (i64, String)> {
        let path = params.get("path").and_then(Value::as_str).ok_or_else(|| {
            (
                super::runtime::INVALID_PARAMS,
                "fs/* takes a path".to_string(),
            )
        })?;
        Ok(Asked {
            path,
            root: params.get("root").and_then(Value::as_str),
            agent: params.get("agent").and_then(Value::as_str),
        })
    }

    /// The folders the plugin may reach for `asked`, real (links followed), and the agent's fence.
    fn folders(
        &self,
        asked: &Asked,
        writing: bool,
    ) -> Result<(Vec<PathBuf>, crate::fence::Fence), (i64, String)> {
        let refused = |why: String| (REFUSED, why);
        let place =
            match asked.agent {
                Some(key) => Some(self.agents.lock().get(key).cloned().ok_or_else(|| {
                    refused(format!("no agent {key:?} was given to this plugin"))
                })?),
                None => None,
            };
        if self.declared.is_empty() {
            return Err(refused(
                "this plugin declares no folder (permissions.files)".into(),
            ));
        }
        let mut folders = Vec::new();
        for f in &self.declared {
            if asked.root.is_some_and(|r| r != f.root) {
                continue;
            }
            let folder = match f.root.as_str() {
                "plugin" => self.folder.clone(),
                "temp" => {
                    let _ = std::fs::create_dir_all(&self.temp);
                    self.temp.clone()
                }
                root => match place.as_ref().and_then(|p| p.root(root)) {
                    Some(folder) => folder.to_path_buf(),
                    None => continue,
                },
            };
            if writing && !f.writes() {
                continue;
            }
            // A root that is gone reaches nothing.
            if let Ok(real) = std::fs::canonicalize(folder) {
                folders.push(real);
            }
        }
        if folders.is_empty() {
            return Err(refused(match (asked.root, asked.agent) {
                (Some(root), _) if !self.declared.iter().any(|f| f.root == root) => {
                    format!("{root} is not in permissions.files")
                }
                (_, None) if !writing => "it names no agent, and no root of its own".to_string(),
                _ if writing => format!("{:?}: it may not write there", asked.path),
                _ => format!("{:?}: no such root for this agent", asked.path),
            }));
        }
        Ok((folders, place.map(|p| p.fence).unwrap_or_default()))
    }

    /// Whether git cannot run anything of the agent's in its folder `path`: its configuration protected and, when it
    /// `commits`, its hooks. What lets `git add` and `git commit` go without asking (`escouade_acp::git`): a module neither
    /// reads `.git` nor runs git. It says two booleans of a folder the plugin was given, and asks for `agentRepo`. A git
    /// that does not answer within a turn says no: the user is asked.
    fn git_protected(&self, params: &Value) -> Result<Value, (i64, String)> {
        let asked = self.asked(params)?;
        if !self.declared.iter().any(|f| f.root == "agentRepo") {
            return Err((REFUSED, "agentRepo is not in permissions.files".into()));
        }
        let same = |cwd: &Path| {
            let cwd = cwd.to_string_lossy();
            escouade_acp::inside(&cwd, asked.path) && escouade_acp::inside(asked.path, &cwd)
        };
        let cwd = self
            .agents
            .lock()
            .values()
            .map(|p| p.cwd.clone())
            .find(|c| same(c));
        let cwd = cwd.ok_or_else(|| {
            (
                REFUSED,
                format!("{:?} is the folder of no agent of this plugin", asked.path),
            )
        })?;
        let commits = params["commits"].as_bool().unwrap_or(true);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let git = escouade_acp::git::repo_protected(&cwd)
                && (!commits || escouade_acp::git::hooks_protected(&cwd));
            let _ = tx.send(git);
        });
        let protected = rx.recv_timeout(GIT_WITHIN).unwrap_or(false);
        Ok(json!({ "protected": protected }))
    }

    /// `{ agent, path }` → `{ fileOnly }`: whether a write of `path`, an absolute path strictly in the folder of the agent
    /// `agent`, reaches a file alone: a file there (not a link), or nothing there and no submodule of git's index at it or
    /// under it, its links leading nowhere out of that folder. What a harness whose rule of a write covers what is under
    /// its path (`agy` 1.5.3, ESC-389) asks before it accepts a write without a card: a module reads no worktree. It names
    /// no root and says one boolean of that agent's own folder; out of it, or in its fence, it is refused. A folder, a
    /// link, anything else, and a git that does not answer within `GIT_WITHIN`, say no: the user is asked.
    fn file_only(&self, params: &Value) -> Result<Value, (i64, String)> {
        let asked = self.asked(params)?;
        let refused = |why: &str| (REFUSED, format!("{:?} {why}", asked.path));
        let key = asked.agent.ok_or_else(|| refused("names no agent"))?;
        let place = self
            .agents
            .lock()
            .get(key)
            .cloned()
            .ok_or_else(|| refused("names no agent given to this plugin"))?;
        let named = named_path(asked.path)
            .filter(|p| p.is_absolute())
            .ok_or_else(|| refused("is no absolute path"))?;
        let (text, cwd) = (named.to_string_lossy(), place.cwd.to_string_lossy());
        if !escouade_acp::inside(&text, &cwd) || escouade_acp::inside(&cwd, &text) {
            return Err(refused("is not strictly in the agent's folder"));
        }
        if place.fence.covers(&text) {
            return Err(refused("is refused to this agent (« Projet privé »)"));
        }
        // Its links followed, it is still in the agent's folder.
        let inside = match (resolved(&named), std::fs::canonicalize(&place.cwd)) {
            (Some(real), Ok(root)) => real.starts_with(&root) && real != root,
            _ => false,
        };
        let only = inside
            && match std::fs::symlink_metadata(&named) {
                Ok(meta) => meta.file_type().is_file(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    no_submodule(&place.cwd, &named)
                }
                Err(_) => false,
            };
        Ok(json!({ "fileOnly": only }))
    }

    /// The real path of what `params` names, links followed as far as it exists: under a root of that agent, out of its
    /// fence, or refused, before anything says whether it exists. With what is left of it to make (`fs/write`).
    fn real(&self, params: &Value, writing: bool) -> Result<PathBuf, (i64, String)> {
        let asked = self.asked(params)?;
        let (folders, fence) = self.folders(&asked, writing)?;
        let refused = |why: &str| (REFUSED, format!("{:?} {why}", asked.path));
        let named = named_path(asked.path).ok_or_else(|| refused("is no path of a root"))?;
        let path = if named.is_absolute() {
            if asked.root.is_some() {
                return Err(refused("is not relative to its root"));
            }
            named
        } else {
            match (asked.root, folders.as_slice()) {
                (Some(_), [folder]) => folder.join(named),
                _ => return Err(refused("is relative, and names no root")),
            }
        };
        let real = resolved(&path).ok_or_else(|| refused("is a link that leads nowhere"))?;
        if !folders.iter().any(|f| real.starts_with(f)) {
            return Err(refused("leads out of the plugin's folders"));
        }
        if fence.covers(&real.to_string_lossy()) {
            return Err(refused("is refused to this agent (« Projet privé »)"));
        }
        Ok(real)
    }

    /// `{ agent?, root?, path, tail? }` → `{ text }`, or `{ dataBase64 }` when it is not UTF-8. With `tail`, a file past the
    /// limit gives its end and `cut` instead of a failure, from a whole character.
    fn read(&self, params: &Value) -> Result<Value, (i64, String)> {
        let real = self.real(params, false)?;
        let at = real.display();
        let meta = std::fs::metadata(&real).map_err(|_| absent(&real))?;
        if !meta.is_file() {
            return Err(absent(&real));
        }
        let cut = meta.len() > self.read_bytes;
        let tail = params.get("tail").and_then(Value::as_bool) == Some(true);
        if cut && !tail {
            return Err((
                FAILED,
                format!("{at} is over {} KiB", self.read_bytes >> 10),
            ));
        }
        let failed = |e: std::io::Error| (FAILED, format!("{at}: {e}"));
        let bytes = if cut {
            let mut bytes = end_of(&real, self.read_bytes).map_err(failed)?;
            let start = bytes.iter().take_while(|b| **b & 0xC0 == 0x80).count();
            bytes.drain(..start);
            bytes
        } else {
            std::fs::read(&real).map_err(failed)?
        };
        let mut read = match String::from_utf8(bytes) {
            Ok(text) => json!({ "text": text }),
            Err(e) => json!({
                "dataBase64": base64::engine::general_purpose::STANDARD.encode(e.as_bytes()),
            }),
        };
        if cut {
            read["cut"] = json!(true);
        }
        Ok(read)
    }

    /// `{ agent?, root?, path, text | dataBase64, append? }` → `{ path, bytes }`; the folders it needs are made.
    fn write(&self, params: &Value) -> Result<Value, (i64, String)> {
        let bytes =
            match (params.get("text"), params.get("dataBase64")) {
                (Some(Value::String(t)), None) => t.clone().into_bytes(),
                (None, Some(Value::String(b))) => base64::engine::general_purpose::STANDARD
                    .decode(b)
                    .map_err(|e| (super::runtime::INVALID_PARAMS, format!("dataBase64: {e}")))?,
                _ => {
                    return Err((
                        super::runtime::INVALID_PARAMS,
                        "fs/write takes text or dataBase64".into(),
                    ))
                }
            };
        if bytes.len() as u64 > MOST_BYTES {
            return Err((FAILED, "fs/write takes 4 MiB at most".into()));
        }
        let real = self.real(params, true)?;
        let at = real.display();
        let failed = |e: std::io::Error| (FAILED, format!("{at}: {e}"));
        if let Some(parent) = real.parent() {
            std::fs::create_dir_all(parent).map_err(failed)?;
        }
        if params.get("append").and_then(Value::as_bool) == Some(true) {
            use std::io::Write as _;
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&real)
                .and_then(|mut f| f.write_all(&bytes))
                .map_err(failed)?;
        } else {
            std::fs::write(&real, &bytes).map_err(failed)?;
        }
        Ok(json!({ "path": plain(&real), "bytes": bytes.len() }))
    }

    /// `{ agent?, root?, path }` → `{ kind, bytes, modified }`: `modified` in milliseconds since 1970.
    fn stat(&self, params: &Value) -> Result<Value, (i64, String)> {
        let real = self.real(params, false)?;
        let meta = std::fs::metadata(&real).map_err(|_| absent(&real))?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);
        Ok(json!({ "kind": kind_of(&meta), "bytes": meta.len(), "modified": modified }))
    }

    /// `{ agent?, root?, path }` → `{ entries: [{ name, kind, bytes }], truncated }`, by name; a link is a `link`, not followed.
    fn list(&self, params: &Value) -> Result<Value, (i64, String)> {
        let real = self.real(params, false)?;
        let dir = std::fs::read_dir(&real).map_err(|_| absent(&real))?;
        let mut entries: Vec<(String, Value)> = dir
            .flatten()
            .map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let meta = std::fs::symlink_metadata(e.path());
                let (kind, bytes) = meta.map_or(("other", 0), |m| {
                    (
                        if m.file_type().is_symlink() {
                            "link"
                        } else {
                            kind_of(&m)
                        },
                        m.len(),
                    )
                });
                let v = json!({ "name": name, "kind": kind, "bytes": bytes });
                (name, v)
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let truncated = entries.len() > LIST_ENTRIES;
        entries.truncate(LIST_ENTRIES);
        let entries: Vec<Value> = entries.into_iter().map(|(_, v)| v).collect();
        Ok(json!({ "entries": entries, "truncated": truncated }))
    }

    /// `{ agent?, root?, path }` → `{ path }`: its real path, in the plain form a harness writes it.
    fn canon(&self, params: &Value) -> Result<Value, (i64, String)> {
        Ok(json!({ "path": plain(&self.real(params, false)?) }))
    }
}

/// The last `bytes` of the file `path`.
fn end_of(path: &Path, bytes: u64) -> std::io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    let start = file.metadata()?.len().saturating_sub(bytes);
    file.seek(SeekFrom::Start(start))?;
    let mut end = Vec::new();
    file.take(bytes).read_to_end(&mut end)?;
    Ok(end)
}

/// The root `params` names, as the journal says it.
fn root_of(params: &Value) -> String {
    params
        .get("root")
        .and_then(Value::as_str)
        .map_or(String::new(), |r| format!(" {r}"))
}

fn absent(path: &Path) -> (i64, String) {
    (NOT_FOUND, format!("no {}", path.display()))
}

fn kind_of(meta: &std::fs::Metadata) -> &'static str {
    if meta.is_file() {
        "file"
    } else if meta.is_dir() {
        "dir"
    } else {
        "other"
    }
}

/// The form a harness writes a path in: without the `\\?\` of Windows.
fn plain(path: &Path) -> String {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => text.strip_prefix(r"\\?\").unwrap_or(&text).to_string(),
    }
}

/// `path` as `fs/*` take it: no `..`, no `~`, no stream of an NTFS file (`a.txt:b`), nothing a string could hide.
fn named_path(path: &str) -> Option<PathBuf> {
    if path.is_empty() || path.contains('\0') || path.starts_with('~') {
        return None;
    }
    let named = PathBuf::from(path);
    named
        .components()
        .all(|c| match c {
            Component::ParentDir => false,
            Component::Normal(part) => !part.to_string_lossy().contains(':'),
            _ => true,
        })
        .then_some(named)
}

/// `path`, its links followed as far as it exists: what is left of it is what `fs/write` would make. A link that leads
/// nowhere gives none: writing through it would land where nothing was judged.
fn resolved(path: &Path) -> Option<PathBuf> {
    let mut rest: Vec<&std::ffi::OsStr> = Vec::new();
    let mut at = path;
    loop {
        if std::fs::symlink_metadata(at).is_ok() {
            let real = std::fs::canonicalize(at).ok()?;
            return Some(rest.iter().rev().fold(real, |p, part| p.join(part)));
        }
        rest.push(at.file_name()?);
        at = at.parent()?;
    }
}

/// Whether git's index of the worktree `cwd` has no submodule (a gitlink, mode 160000) at `path` or under it: no `.git`
/// a write of `path` could make that git would then enter (ESC-389). A folder in no repository has none; a git that
/// fails or does not answer within `GIT_WITHIN` says there may be one.
fn no_submodule(cwd: &Path, path: &Path) -> bool {
    if !cwd.ancestors().any(|d| d.join(".git").exists()) {
        return true;
    }
    let (Some(real), Ok(root)) = (resolved(path), std::fs::canonicalize(cwd)) else {
        return false;
    };
    let Ok(rel) = real.strip_prefix(&root) else {
        return false;
    };
    let rel = rel.to_string_lossy().replace('\\', "/");
    let mut cmd = std::process::Command::new("git");
    cmd.args(["--literal-pathspecs", "-c", "core.fsmonitor=false"])
        .args(["ls-files", "--stage", "-z", "--", &rel])
        .current_dir(&root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(cmd.output());
    });
    match rx.recv_timeout(GIT_WITHIN) {
        Ok(Ok(out)) if out.status.success() => !out
            .stdout
            .split(|b| *b == 0)
            .any(|entry| entry.starts_with(b"160000 ")),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fence::Fence;
    use crate::plugins::runtime::INVALID_PARAMS;
    use serde_json::json;
    use std::cell::RefCell;

    fn folder() -> std::path::PathBuf {
        let base =
            std::env::temp_dir().join(format!("plugin-files-{}", uuid::Uuid::new_v4().simple()));
        let f = base.join("plugin");
        std::fs::create_dir_all(f.join("data")).unwrap();
        std::fs::create_dir_all(f.join(".git")).unwrap();
        std::fs::write(f.join("rules.json"), r#"{"a":1}"#).unwrap();
        std::fs::write(f.join("data/words.txt"), "un\ndeux\n").unwrap();
        std::fs::write(f.join(".env"), "TOKEN=x").unwrap();
        std::fs::write(f.join(".git/config"), "x").unwrap();
        std::fs::write(f.join("escouade-plugin.json"), "{}").unwrap();
        std::fs::write(f.join("big.txt"), "x".repeat(FILE_BYTES as usize + 1)).unwrap();
        std::fs::write(f.join("bin.dat"), [0xff, 0xfe, 0x00]).unwrap();
        std::fs::write(base.join("secret.txt"), "outside").unwrap();
        f
    }

    fn code(f: &Path, path: &str) -> i64 {
        read(f, &json!({ "path": path })).unwrap_err().0
    }

    #[test]
    fn reads_a_file_of_its_folder_and_of_a_subfolder() {
        let f = folder();
        assert_eq!(
            read(&f, &json!({ "path": "rules.json" })).unwrap(),
            json!(r#"{"a":1}"#)
        );
        assert_eq!(
            read(&f, &json!({ "path": "data/words.txt" })).unwrap(),
            json!("un\ndeux\n")
        );
        assert_eq!(
            read(&f, &json!({ "path": "data\\words.txt" })).unwrap(),
            json!("un\ndeux\n")
        );
    }

    #[test]
    fn refuses_any_path_that_leaves_its_folder_or_is_hidden() {
        let f = folder();
        for p in [
            "../secret.txt",
            "data/../../secret.txt",
            "./rules.json",
            "/etc/passwd",
            "\\Windows\\win.ini",
            "C:/Windows/win.ini",
            "C:rules.json",
            ".env",
            ".git/config",
            "data//words.txt",
            "",
            "escouade-plugin.json",
        ] {
            assert_eq!(code(&f, p), REFUSED, "{p}");
        }
        let abs = f.join("rules.json").to_string_lossy().to_string();
        assert_eq!(code(&f, &abs), REFUSED, "{abs}");
        assert_eq!(
            read(&f, &json!({})).unwrap_err().0,
            super::super::runtime::INVALID_PARAMS
        );
    }

    #[test]
    fn says_a_file_missing_too_big_or_not_text() {
        let f = folder();
        assert_eq!(code(&f, "nope.json"), NOT_FOUND);
        assert_eq!(code(&f, "data"), NOT_FOUND);
        assert_eq!(code(&f, "big.txt"), FAILED);
        assert_eq!(code(&f, "bin.dat"), FAILED);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_link_that_leads_out_of_its_folder() {
        let f = folder();
        std::os::unix::fs::symlink(f.join("../secret.txt"), f.join("out.txt")).unwrap();
        std::os::unix::fs::symlink(f.join("rules.json"), f.join("in.json")).unwrap();
        assert_eq!(code(&f, "out.txt"), REFUSED);
        assert_eq!(
            read(&f, &json!({ "path": "in.json" })).unwrap(),
            json!(r#"{"a":1}"#)
        );
    }

    // ---- fs/* ----

    /// A link at `link` to the folder `to`: a junction under Windows, which needs no right.
    fn link_folder(to: &Path, link: &Path) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(to, link).unwrap();
        #[cfg(windows)]
        assert!(std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(to)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success());
    }

    /// Two agents, and the folders around them.
    struct Bed {
        base: PathBuf,
        access: Access,
        told: RefCell<Vec<String>>,
    }

    impl Bed {
        fn new(files: Value) -> Self {
            Self::with(files, json!({}))
        }

        fn with(files: Value, limits: Value) -> Self {
            let base =
                std::env::temp_dir().join(format!("plugin-fs-{}", uuid::Uuid::new_v4().simple()));
            for d in ["plugin", "wt/src", "repo", "home", "other", "fenced"] {
                std::fs::create_dir_all(base.join(d)).unwrap();
            }
            let base = base.canonicalize().unwrap();
            std::fs::write(base.join("plugin/words.txt"), "mots").unwrap();
            std::fs::write(base.join("wt/src/a.txt"), "dans le worktree").unwrap();
            std::fs::write(base.join("repo/HEAD.txt"), "du dépôt").unwrap();
            std::fs::write(base.join("home/auth.json"), "{}").unwrap();
            std::fs::write(base.join("other/b.txt"), "à un autre").unwrap();
            std::fs::write(base.join("fenced/c.txt"), "privé").unwrap();
            let manifest = crate::plugins::manifest::check(
                &json!({
                    "id": "dev.exemple.fichiers",
                    "name": "Fichiers",
                    "version": "1.0.0",
                    "escouade": { "api": "1", "min": "2.2.0" },
                    "permissions": { "files": files },
                    "limits": limits,
                }),
                crate::plugins::manifest::Hand::Third,
            )
            .manifest
            .expect("a manifest that holds");
            let agents: Agents = Arc::default();
            agents.lock().insert(
                "a".into(),
                AgentPlace {
                    cwd: base.join("wt"),
                    repo: Some(base.join("repo")),
                    home: Some(base.join("home")),
                    project: "p".into(),
                    ..AgentPlace::default()
                },
            );
            agents.lock().insert(
                "b".into(),
                AgentPlace {
                    cwd: base.join("other"),
                    ..AgentPlace::default()
                },
            );
            let access = Access::new(&manifest, &base.join("plugin"), base.join("temp"), agents);
            Bed {
                base,
                access,
                told: RefCell::default(),
            }
        }

        fn call(&self, method: &str, params: Value) -> Result<Value, (i64, String)> {
            let told = |_: Kind, l: &str| self.told.borrow_mut().push(l.to_string());
            self.access.serve(method, &params, &told)
        }

        fn refused(&self, method: &str, params: Value) {
            let r = self.call(method, params.clone());
            assert_eq!(
                r.as_ref().err().map(|e| e.0),
                Some(REFUSED),
                "{params}: {r:?}"
            );
        }

        fn path(&self, p: &str) -> String {
            self.base.join(p).to_string_lossy().into_owned()
        }
    }

    fn all_read() -> Value {
        json!([
            { "root": "plugin", "access": "read" },
            { "root": "agent", "access": "read" },
            { "root": "agentRepo", "access": "read" },
            { "root": "agentHome", "access": "read" },
            { "root": "temp", "access": "write" },
        ])
    }

    #[test]
    fn each_root_is_resolved_by_the_host_for_its_agent() {
        let b = Bed::new(all_read());
        let read = |agent: &str, root: &str, path: &str| {
            b.call(
                "fs/read",
                json!({ "agent": agent, "root": root, "path": path }),
            )
            .unwrap()["text"]
                .as_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(read("a", "agent", "src/a.txt"), "dans le worktree");
        assert_eq!(read("a", "agentRepo", "HEAD.txt"), "du dépôt");
        assert_eq!(read("a", "agentHome", "auth.json"), "{}");
        assert_eq!(read("b", "agent", "b.txt"), "à un autre");
        // The plugin's own folder and its temporary one need no agent.
        assert_eq!(
            b.call("fs/read", json!({ "root": "plugin", "path": "words.txt" }))
                .unwrap()["text"],
            "mots"
        );
        b.call(
            "fs/write",
            json!({ "root": "temp", "path": "n/x.txt", "text": "t" }),
        )
        .unwrap();
        assert_eq!(
            b.call("fs/read", json!({ "root": "temp", "path": "n/x.txt" }))
                .unwrap()["text"],
            "t"
        );
        // A path of the host, as a harness writes it, under a root of that agent.
        let path = b.path("wt/src/a.txt");
        assert_eq!(
            b.call("fs/read", json!({ "agent": "a", "path": path }))
                .unwrap()["text"],
            "dans le worktree"
        );
    }

    #[test]
    fn a_harness_asks_whether_the_git_of_its_agent_is_protected() {
        let b = Bed::new(json!([{ "root": "agentRepo", "access": "read" }]));
        let path = b.path("wt");
        let asked = |commits: bool| {
            let params = json!({ "path": path, "commits": commits });
            b.call("fs/gitProtected", params).unwrap()["protected"].clone()
        };
        // No repository: nothing is accepted.
        assert_eq!(asked(false), false);
        // Its own `.git`, which the sandbox protects.
        let git = |args: &[&str]| {
            let done = std::process::Command::new("git")
                .args(args)
                .current_dir(b.base.join("wt"))
                .output()
                .unwrap();
            assert!(done.status.success(), "{done:?}");
        };
        git(&["init", "-q"]);
        assert_eq!(asked(false), true);
        assert_eq!(asked(true), true);
        // Hooks of the worktree, which the agent writes: a commit would run them.
        git(&["config", "core.hooksPath", "src"]);
        assert_eq!(asked(false), true);
        assert_eq!(asked(true), false);
        // The folder of an agent of this plugin, in any form it is written.
        let upper = json!({ "path": path.replace('\\', "/") + "/", "commits": false });
        assert!(b.call("fs/gitProtected", upper).is_ok());
        // Not any other folder.
        let elsewhere = json!({ "path": b.path("repo") });
        assert_eq!(b.call("fs/gitProtected", elsewhere).unwrap_err().0, REFUSED);
    }

    #[test]
    fn git_is_not_read_for_a_plugin_that_does_not_declare_the_repository() {
        let b = Bed::new(json!([{ "root": "agent", "access": "read" }]));
        let params = json!({ "path": b.path("wt") });
        assert_eq!(b.call("fs/gitProtected", params).unwrap_err().0, REFUSED);
    }

    #[test]
    fn a_harness_asks_whether_a_write_reaches_a_file_alone_without_reading_the_worktree() {
        // A plugin that declares its agent's home alone, as Antigravity does (ESC-389).
        let b = Bed::new(json!([{ "root": "agentHome", "access": "read" }]));
        let only = |p: &str| {
            let params = json!({ "agent": "a", "path": b.path(p) });
            b.call("fs/fileOnly", params).unwrap()["fileOnly"].clone()
        };
        // No repository: a file there, or nothing, yes; a folder no.
        assert_eq!(only("wt/src/a.txt"), true);
        assert_eq!(only("wt/src/neuf.txt"), true);
        assert_eq!(only("wt/neuf/b.txt"), true);
        assert_eq!(only("wt/src"), false);
        // A repository with a submodule never initialized (no folder): neither it nor what holds it.
        let git = |args: &[&str]| {
            let done = std::process::Command::new("git")
                .args(args)
                .current_dir(b.base.join("wt"))
                .output()
                .unwrap();
            assert!(done.status.success(), "{done:?}");
        };
        git(&["init", "-q"]);
        let gitlink = "160000,1111111111111111111111111111111111111111,libs/sub";
        git(&["update-index", "--add", "--cacheinfo", gitlink]);
        assert_eq!(only("wt/libs/sub"), false);
        assert_eq!(only("wt/libs"), false);
        assert_eq!(only("wt/libs/autre.txt"), true);
        assert_eq!(only("wt/src/neuf.txt"), true);
        // A name git would read as a pattern is a name.
        assert_eq!(only("wt/libs/[s]ub"), true);
        // Its `.git`, a folder, is no file.
        assert_eq!(only("wt/.git"), false);
        // Strictly in the agent's folder, of an agent of this plugin, or refused.
        for path in [b.path("wt"), b.path("other/b.txt"), b.path("repo/HEAD.txt")] {
            b.refused("fs/fileOnly", json!({ "agent": "a", "path": path }));
        }
        b.refused("fs/fileOnly", json!({ "agent": "a", "path": "src/a.txt" }));
        b.refused(
            "fs/fileOnly",
            json!({ "agent": "z", "path": b.path("wt/src/a.txt") }),
        );
        b.refused("fs/fileOnly", json!({ "path": b.path("wt/src/a.txt") }));
    }

    #[test]
    fn the_agent_tells_its_roots_those_it_declares_only() {
        let b = Bed::new(json!([{ "root": "agent", "access": "read" }]));
        let place = b.access.agents.lock().get("a").cloned().unwrap();
        let roots = b.access.roots_of(&place);
        assert_eq!(
            roots.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["agent"]
        );
        assert_eq!(roots["agent"], b.path("wt"));
    }

    #[test]
    fn a_path_of_another_agent_a_dotdot_or_a_free_path_is_refused_and_said() {
        let b = Bed::new(all_read());
        // Another agent's folder, for this agent.
        b.refused(
            "fs/read",
            json!({ "agent": "a", "path": b.path("other/b.txt") }),
        );
        // `..`, under any spelling.
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agent", "path": "../other/b.txt" }),
        );
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agent", "path": "src/../../other/b.txt" }),
        );
        b.refused(
            "fs/read",
            json!({ "agent": "a", "path": format!("{}/../other/b.txt", b.path("wt")) }),
        );
        // No `~`, no agent that was not given, no root of its own.
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agent", "path": "~/.ssh/id_rsa" }),
        );
        b.refused(
            "fs/read",
            json!({ "agent": "z", "root": "agent", "path": "src/a.txt" }),
        );
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "/", "path": "etc/passwd" }),
        );
        // An absolute path does not go with a root; a relative one needs it.
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agent", "path": b.path("wt/src/a.txt") }),
        );
        b.refused("fs/read", json!({ "agent": "a", "path": "src/a.txt" }));
        // Not even a word of what is out of reach: gone or there, the same refusal.
        b.refused(
            "fs/stat",
            json!({ "agent": "a", "path": b.path("other/nothing.txt") }),
        );
        // A stream of an NTFS file is no path of a root.
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agent", "path": "src/a.txt:hidden" }),
        );
        assert!(b.told.borrow().iter().all(|l| l.contains("refused")));
        assert_eq!(b.told.borrow().len(), 11);
        // Its method and its root: never the path it named.
        assert!(
            !b.told.borrow().iter().any(|l| l.contains("other")),
            "{:?}",
            b.told.borrow()
        );
    }

    #[test]
    fn a_link_that_leaves_its_root_is_refused_one_that_stays_is_followed() {
        let b = Bed::new(all_read());
        link_folder(&b.base.join("other"), &b.base.join("wt/out"));
        link_folder(&b.base.join("wt/src"), &b.base.join("wt/inside"));
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agent", "path": "out/b.txt" }),
        );
        b.refused(
            "fs/list",
            json!({ "agent": "a", "root": "agent", "path": "out" }),
        );
        assert_eq!(
            b.call(
                "fs/read",
                json!({ "agent": "a", "root": "agent", "path": "inside/a.txt" })
            )
            .unwrap()["text"],
            "dans le worktree"
        );
    }

    #[test]
    fn a_write_through_a_link_that_leaves_its_root_touches_nothing() {
        let b = Bed::new(json!([{ "root": "agent", "access": "write" }]));
        link_folder(&b.base.join("other"), &b.base.join("wt/out"));
        b.refused(
            "fs/write",
            json!({ "agent": "a", "root": "agent", "path": "out/new/x.txt", "text": "x" }),
        );
        assert!(!b.base.join("other/new").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_leads_nowhere_is_refused_for_a_write() {
        let b = Bed::new(json!([{ "root": "agent", "access": "write" }]));
        std::os::unix::fs::symlink(b.base.join("other/new.txt"), b.base.join("wt/dangling"))
            .unwrap();
        b.refused(
            "fs/write",
            json!({ "agent": "a", "root": "agent", "path": "dangling", "text": "x" }),
        );
        assert!(!b.base.join("other/new.txt").exists());
    }

    #[test]
    fn the_fence_of_a_private_project_is_held_by_the_host() {
        let b = Bed::new(all_read());
        // Its account's folder holds what another project left there: refused, whatever its root says.
        b.access.agents.lock().get_mut("a").unwrap().fence = Fence::new(vec![b.base.join("home")]);
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agentHome", "path": "auth.json" }),
        );
        // Through a link of its own folder too.
        link_folder(&b.base.join("home"), &b.base.join("wt/account"));
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agent", "path": "account/auth.json" }),
        );
    }

    #[test]
    fn writing_takes_a_root_in_write_and_each_write_is_said() {
        let b = Bed::new(json!([
            { "root": "agent", "access": "write" },
            { "root": "agentHome", "access": "read" },
        ]));
        let w = b
            .call(
                "fs/write",
                json!({ "agent": "a", "root": "agent", "path": "n/e/w.txt", "text": "un" }),
            )
            .unwrap();
        assert_eq!(w["bytes"], 2);
        b.call("fs/write", json!({ "agent": "a", "root": "agent", "path": "n/e/w.txt", "text": "+", "append": true }))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(b.base.join("wt/n/e/w.txt")).unwrap(),
            "un+"
        );
        b.refused(
            "fs/write",
            json!({ "agent": "a", "root": "agentHome", "path": "auth.json", "text": "x" }),
        );
        assert_eq!(
            std::fs::read_to_string(b.base.join("home/auth.json")).unwrap(),
            "{}"
        );
        let told = b.told.borrow();
        assert!(told[0].contains("wrote 2 bytes"), "{told:?}");
        assert!(told[2].contains("refused"), "{told:?}");
    }

    #[test]
    fn nothing_declared_nothing_reached() {
        let b = Bed::new(json!([]));
        b.refused(
            "fs/read",
            json!({ "agent": "a", "root": "agent", "path": "src/a.txt" }),
        );
        b.refused("fs/list", json!({ "root": "temp", "path": "." }));
    }

    #[test]
    fn a_read_stops_at_256_kib_or_at_its_limit_and_never_past_4_mib() {
        let b = Bed::new(json!([{ "root": "agent", "access": "read" }]));
        std::fs::write(b.base.join("wt/big"), vec![b'x'; READ_BYTES as usize + 1]).unwrap();
        let big = json!({ "agent": "a", "root": "agent", "path": "big" });
        assert_eq!(b.call("fs/read", big.clone()).unwrap_err().0, FAILED);
        let files = json!([{ "root": "agent", "access": "read" }]);
        let b = Bed::with(files.clone(), json!({ "fileBytes": 1 << 19 }));
        std::fs::write(b.base.join("wt/big"), vec![b'x'; READ_BYTES as usize + 1]).unwrap();
        assert!(b.call("fs/read", big.clone()).is_ok());
        let b = Bed::with(files, json!({ "fileBytes": 1 << 30 }));
        std::fs::write(b.base.join("wt/big"), vec![b'x'; MOST_BYTES as usize + 1]).unwrap();
        assert_eq!(b.call("fs/read", big).unwrap_err().0, FAILED);
    }

    #[test]
    fn stat_list_and_canon_say_what_is_there() {
        let b = Bed::new(all_read());
        let at = |path: &str| json!({ "agent": "a", "root": "agent", "path": path });
        let s = b.call("fs/stat", at("src/a.txt")).unwrap();
        assert_eq!(s["kind"], "file");
        assert_eq!(s["bytes"], "dans le worktree".len());
        assert_eq!(b.call("fs/stat", at("src")).unwrap()["kind"], "dir");
        assert_eq!(b.call("fs/stat", at("nothing")).unwrap_err().0, NOT_FOUND);
        std::fs::create_dir_all(b.base.join("wt/d")).unwrap();
        let names: Vec<String> = b.call("fs/list", at(".")).unwrap()["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                format!(
                    "{} {}",
                    e["name"].as_str().unwrap(),
                    e["kind"].as_str().unwrap()
                )
            })
            .collect();
        assert_eq!(names, vec!["d dir", "src dir"]);
        let canon = b.call("fs/canon", at("src/a.txt")).unwrap();
        let path = canon["path"].as_str().unwrap();
        assert!(!path.starts_with(r"\\?\"), "{path}");
        assert!(Path::new(path).ends_with("wt/src/a.txt"), "{path}");
    }

    #[test]
    fn a_file_that_is_no_text_comes_in_base64_and_a_write_takes_it_back() {
        let b = Bed::new(json!([{ "root": "agent", "access": "write" }]));
        std::fs::write(b.base.join("wt/bin"), [0xff, 0xfe, 0x00]).unwrap();
        let r = b
            .call(
                "fs/read",
                json!({ "agent": "a", "root": "agent", "path": "bin" }),
            )
            .unwrap();
        let data = r["dataBase64"].as_str().unwrap();
        b.call(
            "fs/write",
            json!({ "agent": "a", "root": "agent", "path": "copy", "dataBase64": data }),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(b.base.join("wt/copy")).unwrap(),
            [0xff, 0xfe, 0x00]
        );
        let bad = json!({ "agent": "a", "root": "agent", "path": "x" });
        assert_eq!(b.call("fs/write", bad).unwrap_err().0, INVALID_PARAMS);
    }

    #[test]
    fn the_repository_of_a_worktree_is_found_from_its_git_file() {
        let base =
            std::env::temp_dir().join(format!("plugin-repo-{}", uuid::Uuid::new_v4().simple()));
        let repo = base.join("repo");
        let gitdir = repo.join(".git/worktrees/wt");
        std::fs::create_dir_all(&gitdir).unwrap();
        std::fs::write(gitdir.join("commondir"), "../..\n").unwrap();
        let wt = repo.join(".claude/worktrees/wt");
        std::fs::create_dir_all(wt.join("deep")).unwrap();
        std::fs::write(wt.join(".git"), "gitdir: ../../../.git/worktrees/wt\n").unwrap();
        let real = std::fs::canonicalize(&repo).unwrap();
        assert_eq!(repo_of(&wt), Some(real.clone()));
        assert_eq!(repo_of(&wt.join("deep")), Some(real.clone()));
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();
        assert_eq!(repo_of(&wt), Some(real));
        assert_eq!(repo_of(&repo), Some(repo.clone()));
    }

    #[test]
    fn a_tail_gives_the_end_of_what_is_past_the_limit_from_a_whole_character() {
        let bed = Bed::with(
            json!([{ "root": "agent", "access": "read" }]),
            json!({ "fileBytes": 10 }),
        );
        // Ten bytes back falls inside a « é ».
        std::fs::write(bed.base.join("wt/long.log"), "1234567890éabcdefghi").unwrap();
        let at = json!({ "agent": "a", "root": "agent", "path": "long.log" });
        let err = bed.access.serve("fs/read", &at, &|_, _| {}).unwrap_err();
        assert_eq!(err.0, FAILED, "without `tail`, as before");
        let mut tail = at.clone();
        tail["tail"] = json!(true);
        let read = bed.access.serve("fs/read", &tail, &|_, _| {}).unwrap();
        assert_eq!(read["cut"], true);
        assert_eq!(read["text"], "abcdefghi");
        // A file under the limit is whole, and says nothing.
        std::fs::write(bed.base.join("wt/short.log"), "court").unwrap();
        let short = json!({ "agent": "a", "root": "agent", "path": "short.log", "tail": true });
        let read = bed.access.serve("fs/read", &short, &|_, _| {}).unwrap();
        assert_eq!(read, json!({ "text": "court" }));
    }
}
