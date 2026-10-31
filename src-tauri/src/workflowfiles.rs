//! The configuration of a project's workflows, shared with its team (ESC-195): every workflow
//! detected is a file of its own, `.escouade/workflows/<name>.yml` beside the shared tickets
//! (`sharing`), with the definition the reading found and, apart, the user's corrections
//! (`fixes`, `workflowfix`). Escouade writes these files and keeps them up to date; the user commits
//! them with their work, never Escouade. They are read when the folder changes (a pull, another
//! branch), and the folder is what counts: a teammate whose machine does not read a workflow
//! (no GSD there) still sees and follows it.
//!
//! A reading of this machine adds the workflows it finds and updates those whose definition
//! changed; it never removes a file. One the user removed (its file deleted by hand, or
//! « Supprimer » in the window) is never written again (ESC-309, `Removals`): this machine
//! remembers it, and proposes it again only when what found it changes. A workflow is set apart by « Ce n'est pas un workflow » (its
//! file kept, marked, so that no reading makes it again) or taken out by « Retirer du dépôt ».
//! Nothing of one machine goes in a file: no date, no fingerprint, no absolute path, no state of
//! the work (`StateFile::exists`, `says`); what tells whether a reading is stale stays in the
//! data folder (`workflows.json`). The text is written the same way every time: a reading that
//! changes nothing touches no file.
//!
//! Only the workflows of the repository's own configuration (`.claude/`, `.agents/`, `.github/`,
//! `.opencode/`, `.cursor/`, a `PROMPT.md`) go there (`in_repo`: where its entry is, even when
//! a step calls a skill of the user's). Those of the user's own folders (`~/.claude`, `~/.codex`,
//! GSD or Superpowers installed there) are read, shown and followed all the same, but their files
//! stay in the data folder (`~/.escouade/projects/<id>/workflows/`), in the same format, never in
//! the repository. « Sur cette machine » (`Project::workflows_in_repo` off) keeps the repository's
//! there too, for a project where nothing is to be written in the repository.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::model::Project;
use crate::projectfile::{plain_safe, tree, Node, N};
use crate::workflow::{Scan, Workflow};
use crate::workflowfix::{Fix, Fixes};

/// Where the files are in the project, beside the shared tickets' (`sharing::DIR`).
pub const DIR: &str = ".escouade/workflows";
pub const VERSION: u32 = 1;
/// Where the site serves the schema (`schemas/escouade-workflow.schema.json` of the repository).
pub const SCHEMA_URL: &str = "https://escouade.dev/schemas/escouade-workflow.schema.json";
/// A file bigger than this is not read.
const MAX_FILE: u64 = 2 * 1024 * 1024;

/// A file of the folder, as the window says it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FileInfo {
    /// The workflow's id.
    pub id: String,
    /// From the project's folder (`.escouade/workflows/gsd.yml`), or the data folder's.
    pub path: String,
    /// This machine's reading finds it too; else it is the team's alone (« Retirer du dépôt »).
    pub local: bool,
    /// Its file is in the repository (`.escouade/workflows/`); else on this machine alone.
    pub in_repo: bool,
    /// What the last reading changed of it, when it did.
    pub updated: Option<String>,
}

/// A file that could not be read: its path and why.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Unreadable {
    pub path: String,
    pub why: String,
}

/// A file: the workflow as read, and the user's corrections of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    pub workflow: Workflow,
    pub fixes: Fix,
}

/// The file name of a workflow's id: letters, digits, `.`, `_` and `-` alone, so that no system
/// refuses it (`|claude|gsd` → `claude-gsd`, `api|claude|gsd` → `api-claude-gsd`).
pub fn safe_name(id: &str) -> String {
    let mut out = String::new();
    for c in id.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches(['-', '.']).to_string();
    let out = if out.is_empty() {
        "workflow".to_string()
    } else {
        out
    };
    // A name Windows keeps for itself is one no more, with or without an extension (`con.yml` is
    // the device too): the stem before the first dot counts.
    let (stem, rest) = out
        .split_once('.')
        .map_or((out.as_str(), ""), |(s, _)| (s, &out[s.len()..]));
    if reserved_stem(stem) {
        return format!("{stem}-workflow{rest}").chars().take(80).collect();
    }
    out.chars().take(80).collect()
}

/// A name Windows keeps for devices: `CON`, `PRN`, `AUX`, `NUL`, `COM0`–`COM9`, `LPT0`–`LPT9`,
/// and `CONIN$` / `CONOUT$` (whose `$` `safe_name` never lets through, so `conin` and `conout`
/// stand for them). Lower case.
pub fn reserved_stem(stem: &str) -> bool {
    matches!(stem, "con" | "prn" | "aux" | "nul" | "conin" | "conout")
        || ((stem.starts_with("com") || stem.starts_with("lpt"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit())
}

fn is_absolute(s: &str) -> bool {
    s.starts_with('/') || s.starts_with('\\') || (s.len() >= 2 && s.as_bytes()[1] == b':')
}

/// Why the repository's `.escouade/<x>` folder `dir` is not to be read or written, if it is not: it
/// (or `.escouade`) is a link, or its real path leaves the project's folder. A cloned repository
/// can hold such a link, to somewhere the user never meant. None when it is not there yet.
pub fn unsafe_dir(dir: &Path) -> Option<String> {
    let parent = dir.parent()?;
    if parent.file_name()? != ".escouade" {
        return None;
    }
    for p in [parent, dir] {
        if std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()) {
            return Some(tr!(
                "le dossier {p} est un lien symbolique : ignoré",
                "the folder {p} is a symbolic link: ignored",
                p = p.display()
            ));
        }
    }
    let project = parent.parent()?;
    let (real, home) = (dir.canonicalize().ok()?, project.canonicalize().ok()?);
    (!real.starts_with(&home)).then(|| {
        tr!(
            "le dossier {p} sort du projet : ignoré",
            "the folder {p} leaves the project: ignored",
            p = dir.display()
        )
    })
}

/// An absolute path (one the reading could not say from the project or `~`) is only its name.
fn portable_path(s: &str) -> String {
    if is_absolute(s) {
        s.replace('\\', "/")
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string()
    } else {
        s.to_string()
    }
}

fn portable_value(key: &str, v: &mut Value) {
    match v {
        Value::Object(m) => {
            // The state of the work is this machine's, now: never shared.
            m.remove("exists");
            m.remove("says");
            for (k, x) in m.iter_mut() {
                portable_value(k, x);
            }
        }
        Value::Array(items) => {
            for x in items.iter_mut() {
                portable_value(key, x);
            }
        }
        Value::String(s) if matches!(key, "path" | "prompt" | "reads" | "writes") => {
            *s = portable_path(s);
        }
        _ => {}
    }
}

/// The definition of a workflow as the file keeps it: nothing of one machine.
pub fn portable(w: &Workflow) -> Value {
    let mut v = serde_json::to_value(w).unwrap_or_default();
    portable_value("", &mut v);
    v
}

// ---------- writing ----------

/// The order of the keys in a file; the others after, alphabetically.
const ORDER: &[&str] = &[
    "version",
    "workflow",
    "fixes",
    "id",
    "name",
    "kind",
    "harness",
    "repo",
    "command",
    "role",
    "scope",
    "path",
    "description",
    "order",
    "enters",
    "agents",
    "reads",
    "writes",
    "guessed",
    "also",
    "from",
    "to",
    "back",
    "condition",
    "max",
    "doors",
    "steps",
    "transitions",
    "stateFiles",
    "exit",
    "promptLoop",
    "refined",
    "dismissed",
    "removed",
    "signals",
];

fn ordered(m: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut items: Vec<(&String, &Value)> = m.iter().collect();
    items.sort_by_key(|(k, _)| {
        (
            ORDER.iter().position(|o| o == k).unwrap_or(ORDER.len()),
            (*k).clone(),
        )
    });
    items
}

fn scalar(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) if plain_safe(s) => s.clone(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_default(),
        Value::Array(_) => "[]".into(),
        Value::Object(_) => "{}".into(),
    }
}

fn is_leaf(v: &Value) -> bool {
    match v {
        Value::Array(a) => a.is_empty(),
        Value::Object(m) => m.is_empty(),
        _ => true,
    }
}

/// A mapping at `indent`; `dash`: the first key goes on the line of a list's `- `.
fn emit_map(m: &Map<String, Value>, indent: usize, dash: bool, out: &mut String) {
    for (i, (k, v)) in ordered(m).into_iter().enumerate() {
        let lead = if dash && i == 0 {
            format!("{}- ", " ".repeat(indent - 2))
        } else {
            " ".repeat(indent)
        };
        if is_leaf(v) {
            out.push_str(&format!("{lead}{k}: {}\n", scalar(v)));
            continue;
        }
        out.push_str(&format!("{lead}{k}:\n"));
        emit(v, indent + 2, out);
    }
}

fn emit(v: &Value, indent: usize, out: &mut String) {
    match v {
        Value::Object(m) => emit_map(m, indent, false, out),
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::Object(m) if !m.is_empty() => emit_map(m, indent + 2, true, out),
                    Value::Array(a) if !a.is_empty() => {
                        out.push_str(&format!("{}-\n", " ".repeat(indent)));
                        emit(item, indent + 2, out);
                    }
                    _ => out.push_str(&format!("{}- {}\n", " ".repeat(indent), scalar(item))),
                }
            }
        }
        _ => out.push_str(&format!("{}{}\n", " ".repeat(indent), scalar(v))),
    }
}

/// The file's text, the same way every time.
pub fn write(doc: &Doc) -> String {
    let mut out = String::new();
    out.push_str(
        "# A workflow of the project, read by Escouade: https://escouade.dev/en/docs/workflows\n",
    );
    out.push_str(
        "# Escouade keeps it up to date; commit it with your work, Escouade never does.\n",
    );
    out.push_str(&format!("# yaml-language-server: $schema={SCHEMA_URL}\n"));
    let mut top = Map::new();
    top.insert("version".into(), json!(VERSION));
    top.insert("workflow".into(), portable(&doc.workflow));
    let mut fixes = serde_json::to_value(&doc.fixes).unwrap_or_default();
    if let Value::Object(m) = &mut fixes {
        m.remove("workflow");
        if !m.is_empty() {
            top.insert("fixes".into(), fixes);
        }
    }
    emit_map(&top, 0, false, &mut out);
    out
}

// ---------- reading ----------

/// A node as JSON: a mapping an object, a list an array; unquoted, `null` and `~` null, `true`
/// and `false` booleans, a whole number a number; the rest texts.
fn json_of(n: &Node) -> Value {
    match &n.v {
        N::Scalar { null: true, .. } | N::Other => Value::Null,
        N::Scalar {
            text,
            quoted: false,
            ..
        } => match text.as_str() {
            "true" => json!(true),
            "false" => json!(false),
            t => t
                .parse::<i64>()
                .map(|i| json!(i))
                .unwrap_or_else(|_| json!(t)),
        },
        N::Scalar { text, .. } => json!(text),
        N::Map(entries) => Value::Object(
            entries
                .iter()
                .map(|(k, _, v)| (k.clone(), json_of(v)))
                .collect(),
        ),
        N::Seq(items) => Value::Array(items.iter().map(json_of).collect()),
    }
}

/// A file's text: its workflow and its corrections, or why it cannot be read.
pub fn read(text: &str) -> Result<Doc, String> {
    let t = tree(text).map_err(|w| w.text)?;
    let v = json_of(&t);
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    if version == 0 {
        return Err(tr!(
            "« version » manque ou n'est pas un entier",
            "“version” is missing or not a whole number"
        ));
    }
    if version > VERSION as u64 {
        return Err(tr!(
            "fait pour une version plus récente d'Escouade (version {version})",
            "made for a newer version of Escouade (version {version})"
        ));
    }
    let w = v
        .get("workflow")
        .cloned()
        .ok_or_else(|| tr!("« workflow » manque", "“workflow” is missing"))?;
    let workflow: Workflow = serde_json::from_value(w).map_err(|e| format!("workflow: {e}"))?;
    if workflow.id.trim().is_empty() {
        return Err(tr!("« workflow.id » est vide", "“workflow.id” is empty"));
    }
    let mut fixes = match v.get("fixes") {
        None | Some(Value::Null) => Fix::default(),
        Some(f) => serde_json::from_value(f.clone()).map_err(|e| format!("fixes: {e}"))?,
    };
    fixes.workflow = workflow.id.clone();
    Ok(Doc { workflow, fixes })
}

// ---------- the folder ----------

/// Whether a workflow belongs to the repository's configuration: its entry (its first step, the
/// prompt of a loop of one prompt) is a file of the repository, not of the user's own folders
/// (`~/…`). A step may still call a skill of the user's: its name is kept.
pub fn in_repo(w: &Workflow) -> bool {
    let entry = match &w.prompt_loop {
        Some(p) if w.kind == crate::workflow::Kind::Prompt => p.prompt.clone(),
        _ => w
            .steps
            .iter()
            .min_by_key(|s| s.order)
            .map(|s| s.path.clone())
            .unwrap_or_default(),
    };
    !entry.starts_with('~') && !is_absolute(&entry)
}

/// Where a project's workflows' files go: those of the repository's configuration (`repo`: in
/// its folder beside the shared tickets, in its first repository when it has several and its
/// folder is none, as `sharing::dir_of`; or the data folder's when kept on this machine), and
/// those of the user's own folders (`local`, always the data folder's).
#[derive(Debug, Clone, PartialEq)]
pub struct Places {
    pub repo: PathBuf,
    pub local: PathBuf,
}

pub fn places(project: &Project, data: &crate::paths::DataDir) -> Places {
    let local = data.project_files(&project.id).join("workflows");
    let repo = if project.workflows_in_repo {
        crate::sharing::dir_of(project)
            .parent()
            .map(|p| p.join("workflows"))
            .unwrap_or_else(|| Path::new(&project.path).join(DIR))
    } else {
        local.clone()
    };
    Places { repo, local }
}

/// The files of both places, as one folder (a workflow in both is told, its second file left).
pub fn load_all(p: &Places) -> Folder {
    let mut out = load(&p.repo);
    if p.local != p.repo {
        let more = load(&p.local);
        for (path, doc) in more.docs {
            if out.doc(&doc.workflow.id).is_some() {
                out.unreadable.push((
                    path,
                    tr!(
                        "un autre fichier a déjà ce workflow ({id})",
                        "another file has this workflow already ({id})",
                        id = doc.workflow.id
                    ),
                ));
            } else {
                out.docs.push((path, doc));
            }
        }
        out.unreadable.extend(more.unreadable);
        for (name, text) in more.texts {
            out.texts.insert(format!("local/{name}"), text);
        }
    }
    out
}

/// `merge_reading` in both places: each workflow read goes where it belongs (`in_repo`), the
/// repository's only when `repo_ok` (its folder is there), those the user removed left out
/// (`Removals`, which it keeps up to date).
pub fn merge_reading_all(
    p: &Places,
    folder: &Folder,
    read: &[Workflow],
    repo_ok: bool,
    removals: &mut Removals,
) -> Vec<(String, String)> {
    if repo_ok {
        removals.note_gone(folder);
    }
    let read: Vec<Workflow> = read
        .iter()
        .filter(|w| !removals.removed.contains_key(&w.id))
        .cloned()
        .collect();
    let (ours, theirs): (Vec<Workflow>, Vec<Workflow>) = read.into_iter().partition(in_repo);
    let mut out = Vec::new();
    if repo_ok {
        out.extend(merge_reading(&p.repo, folder, &ours));
    }
    let again;
    let folder = if out.is_empty() {
        folder
    } else {
        again = load_all(p);
        &again
    };
    let more = merge_reading(&p.local, folder, &theirs);
    let again;
    let folder = if more.is_empty() {
        folder
    } else {
        again = load_all(p);
        &again
    };
    out.extend(more);
    if repo_ok {
        removals.known = folder
            .docs
            .iter()
            .map(|(_, d)| (d.workflow.id.clone(), Gone::of(d)))
            .collect();
    }
    out
}

// ---------- the workflows removed ----------

/// A workflow as this machine remembers it: what found it (the fingerprint of its reading) and its
/// name, for the window.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Gone {
    pub fingerprint: String,
    pub name: String,
}

impl Gone {
    pub fn of(doc: &Doc) -> Self {
        Gone {
            // Refined by an agent (ESC-196): the reading it was refined from.
            fingerprint: doc
                .workflow
                .refined
                .clone()
                .unwrap_or_else(|| fingerprint_of(&doc.workflow)),
            name: doc.workflow.name.clone(),
        }
    }
}

/// The workflows the user removed from a project, kept on this machine (ESC-309,
/// `~/.escouade/projects/<id>/workflows-removed.json`): a reading writes none of them again. And
/// those that had a file at the last reading: one whose file is gone since was removed by hand.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Removals {
    pub known: BTreeMap<String, Gone>,
    pub removed: BTreeMap<String, Gone>,
}

/// A workflow removed, as the window says it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct RemovedInfo {
    pub id: String,
    pub name: String,
    /// What found it changed since: proposed again, not written.
    pub changed: bool,
}

pub fn removals_file(data: &crate::paths::DataDir, project_id: &str) -> PathBuf {
    data.project_files(project_id)
        .join("workflows-removed.json")
}

impl Removals {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Written when it changed from `before`.
    pub fn save(&self, path: &Path, before: &Removals) {
        if self == before {
            return;
        }
        let write = || -> anyhow::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            crate::paths::write_atomic(path, &serde_json::to_vec_pretty(self)?)?;
            Ok(())
        };
        if let Err(e) = write() {
            log::warn!("{}: {e:#}", path.display());
        }
    }

    /// The workflows whose file went since the last reading are removed; those whose file is there
    /// (again: a pull, another branch) are the project's, no longer removed. A file that cannot
    /// be read, or a folder refused, tells nothing.
    pub fn note_gone(&mut self, folder: &Folder) {
        let refused = folder
            .unreadable
            .iter()
            .any(|(p, _)| p.extension().is_none_or(|x| x != "yml"));
        if !refused {
            for (id, g) in std::mem::take(&mut self.known) {
                let own = format!("{}.yml", safe_name(&id));
                let unreadable = folder
                    .unreadable
                    .iter()
                    .any(|(p, _)| p.file_name().is_some_and(|n| n.to_string_lossy() == own));
                if folder.doc(&id).is_none() && !unreadable {
                    self.removed.insert(id, g);
                } else {
                    self.known.insert(id, g);
                }
            }
        }
        for (_, d) in &folder.docs {
            self.removed.remove(&d.workflow.id);
        }
    }

    /// Removes a workflow (« Supprimer »), or keeps one removed whose finding changed as it is
    /// now (« Laisser supprimé »): `now`, the reading's, when there is one.
    pub fn remove(&mut self, id: &str, doc: Option<&Doc>, now: Option<&Workflow>) {
        let mut g = doc
            .map(Gone::of)
            .or_else(|| self.removed.get(id).cloned())
            .or_else(|| self.known.get(id).cloned())
            .unwrap_or_default();
        if let Some(w) = now {
            g.fingerprint = fingerprint_of(w);
            if g.name.is_empty() {
                g.name = w.name.clone();
            }
        }
        self.known.remove(id);
        self.removed.insert(id.to_string(), g);
    }

    /// The workflows removed, as the window says them: `read`, this machine's last reading,
    /// tells whether what found one changed.
    pub fn listed(&self, read: &[Workflow]) -> Vec<RemovedInfo> {
        self.removed
            .iter()
            .map(|(id, g)| RemovedInfo {
                id: id.clone(),
                name: g.name.clone(),
                changed: read
                    .iter()
                    .find(|w| w.id == *id)
                    .is_some_and(|w| fingerprint_of(w) != g.fingerprint),
            })
            .collect()
    }
}

/// The files of a folder as read: each one's path, and what it holds or why it cannot be read.
#[derive(Debug, Default)]
pub struct Folder {
    pub docs: Vec<(PathBuf, Doc)>,
    pub unreadable: Vec<(PathBuf, String)>,
    /// Every file's text, for the fingerprint of the folder.
    pub texts: BTreeMap<String, String>,
}

impl Folder {
    pub fn doc(&self, id: &str) -> Option<&(PathBuf, Doc)> {
        self.docs.iter().find(|(_, d)| d.workflow.id == id)
    }

    /// The corrections the files hold, by workflow, in the order of their ids.
    pub fn fixes(&self) -> Fixes {
        let mut out: Fixes = self
            .docs
            .iter()
            .map(|(_, d)| d.fixes.clone())
            .filter(|f| !f.is_empty())
            .collect();
        out.sort_by(|a, b| a.workflow.cmp(&b.workflow));
        out
    }

    /// The fingerprint of what the folder holds.
    pub fn fingerprint(&self) -> String {
        crate::sharing::fingerprint(&serde_json::to_string(&self.texts).unwrap_or_default())
    }
}

/// Reads the `.yml` files of `dir` (none when it is not there).
pub fn load(dir: &Path) -> Folder {
    let mut out = Folder::default();
    if let Some(why) = unsafe_dir(dir) {
        out.unreadable.push((dir.to_path_buf(), why));
        return out;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .collect();
    paths.sort();
    for path in paths {
        // A link is never followed: it can point at any file of the machine.
        let kind = std::fs::symlink_metadata(&path).map(|m| m.file_type());
        if kind.as_ref().is_ok_and(|k| k.is_symlink()) {
            out.unreadable
                .push((path, tr!("lien symbolique ignoré", "symbolic link ignored")));
            continue;
        }
        if !kind.is_ok_and(|k| k.is_file()) {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_FILE) {
            out.unreadable.push((
                path,
                tr!(
                    "fichier trop gros (plus de 2 Mo)",
                    "file too big (over 2 MB)"
                ),
            ));
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                out.unreadable.push((path, e.to_string()));
                continue;
            }
        };
        out.texts.insert(name, text.clone());
        match read(&text) {
            Ok(doc) if out.doc(&doc.workflow.id).is_some() => out.unreadable.push((
                path,
                tr!(
                    "un autre fichier a déjà ce workflow ({id})",
                    "another file has this workflow already ({id})",
                    id = doc.workflow.id
                ),
            )),
            Ok(doc) => out.docs.push((path, doc)),
            Err(why) => out.unreadable.push((path, why)),
        }
    }
    out
}

fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(why) = path.parent().and_then(unsafe_dir) {
        return Err(std::io::Error::other(why));
    }
    if std::fs::read_to_string(path).ok().as_deref() == Some(text) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::paths::write_atomic(path, text.as_bytes()).map_err(std::io::Error::other)
}

/// The file a new workflow goes to: its safe name, or the next one free.
fn new_path(dir: &Path, folder: &Folder, id: &str) -> PathBuf {
    let base = safe_name(id);
    let taken = |p: &Path| {
        folder.docs.iter().any(|(q, _)| q == p)
            || folder.unreadable.iter().any(|(q, _)| q == p)
            || p.exists()
    };
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{base}.yml")
        } else {
            format!("{base}-{n}.yml")
        };
        let p = dir.join(name);
        if !taken(&p) {
            return p;
        }
        n += 1;
    }
}

/// The fingerprint of a reading's definition (what `Workflow::refined` keeps).
pub fn fingerprint_of(w: &Workflow) -> String {
    let mut w = w.clone();
    w.refined = None;
    crate::sharing::fingerprint(&portable(&w).to_string())
}

/// What changed of a definition, said simply: the steps added and taken out.
fn change_of(old: &Workflow, new: &Workflow) -> String {
    let names = |w: &Workflow| w.steps.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
    let (a, b) = (names(old), names(new));
    let added: Vec<&String> = b.iter().filter(|n| !a.contains(n)).collect();
    let gone: Vec<&String> = a.iter().filter(|n| !b.contains(n)).collect();
    let list = |v: &[&String]| v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
    match (added.is_empty(), gone.is_empty()) {
        (true, true) => tr!("sa définition a changé", "its definition changed"),
        (false, true) => tr!("étapes ajoutées : {}", "steps added: {}", list(&added)),
        (true, false) => tr!("étapes disparues : {}", "steps gone: {}", list(&gone)),
        (false, false) => tr!(
            "étapes ajoutées : {} ; disparues : {}",
            "steps added: {}; gone: {}",
            list(&added),
            list(&gone)
        ),
    }
}

/// What a reading of this machine does to the folder: the workflows it finds that no file has
/// are written, those whose definition changed are written again with their corrections; no
/// file is removed. Returns what changed, by workflow id. A workflow whose file cannot be read is
/// left as it is (the window says why).
pub fn merge_reading(dir: &Path, folder: &Folder, read: &[Workflow]) -> Vec<(String, String)> {
    let mut changed = Vec::new();
    let mut seen = Folder {
        docs: folder.docs.clone(),
        unreadable: folder.unreadable.clone(),
        texts: BTreeMap::new(),
    };
    for w in read {
        match seen.doc(&w.id).cloned() {
            Some((path, doc)) => {
                if portable(&doc.workflow) == portable(w) {
                    continue;
                }
                // Refined from this very reading (ESC-196): what the agent made stands.
                if doc.workflow.refined.as_deref() == Some(fingerprint_of(w).as_str()) {
                    continue;
                }
                let next = Doc {
                    workflow: w.clone(),
                    fixes: doc.fixes.clone(),
                };
                if write_file(&path, &write(&next)).is_ok() {
                    changed.push((w.id.clone(), change_of(&doc.workflow, w)));
                }
            }
            None => {
                // Its file is there but unreadable: left for the user to mend.
                let own = dir.join(format!("{}.yml", safe_name(&w.id)));
                if seen.unreadable.iter().any(|(p, _)| *p == own) {
                    continue;
                }
                let path = new_path(dir, &seen, &w.id);
                let doc = Doc {
                    workflow: w.clone(),
                    fixes: Fix {
                        workflow: w.id.clone(),
                        ..Default::default()
                    },
                };
                if write_file(&path, &write(&doc)).is_ok() {
                    changed.push((w.id.clone(), String::new()));
                    seen.docs.push((path, doc));
                }
            }
        }
    }
    changed
}

/// Writes workflows an agent refined (ESC-196) into their files, the user's corrections kept:
/// what changed of each, by workflow id; a file that already says it is not touched.
pub fn set_workflows(folder: &Folder, refined: &[Workflow]) -> Vec<(String, String)> {
    let mut changed = Vec::new();
    for w in refined {
        let Some((path, doc)) = folder.doc(&w.id) else {
            continue;
        };
        let next = Doc {
            workflow: w.clone(),
            fixes: doc.fixes.clone(),
        };
        let text = write(&next);
        if text != write(doc) && write_file(path, &text).is_ok() {
            changed.push((w.id.clone(), change_of(&doc.workflow, w)));
        }
    }
    changed
}

/// Writes the user's corrections into the files of their workflows (those without any lose
/// theirs). A correction of a workflow no file holds is left out.
pub fn set_fixes(folder: &Folder, fixes: &[Fix]) -> std::io::Result<()> {
    for (path, doc) in &folder.docs {
        let mut fix = fixes
            .iter()
            .find(|f| f.workflow == doc.workflow.id)
            .cloned()
            .unwrap_or_default();
        fix.workflow = doc.workflow.id.clone();
        if fix == doc.fixes {
            continue;
        }
        let next = Doc {
            workflow: doc.workflow.clone(),
            fixes: fix,
        };
        write_file(path, &write(&next))?;
    }
    Ok(())
}

/// The files of the repository's workflows in `from` moved to `to` (« Où garder » changed), none
/// lost nor doubled: a workflow `to` has already is written over with `from`'s, the latest the
/// user had. Those of the user's own folders stay where they are.
pub fn move_all(from: &Path, to: &Path) -> std::io::Result<()> {
    if from == to {
        return Ok(());
    }
    let src = load(from);
    let dest = load(to);
    for (path, doc) in src.docs.iter().filter(|(_, d)| in_repo(&d.workflow)) {
        let target = dest
            .doc(&doc.workflow.id)
            .map(|(p, _)| p.clone())
            .unwrap_or_else(|| new_path(to, &dest, &doc.workflow.id));
        write_file(&target, &write(doc))?;
        std::fs::remove_file(path)?;
    }
    // A folder left empty goes.
    let _ = std::fs::remove_dir(from);
    Ok(())
}

/// The workflows of the folder, as the project shows and follows them: the definitions of the
/// files, the state of their files read on this machine, beside what `local` (this machine's
/// reading) says of the rest.
pub fn scan_of(
    project: &Project,
    folder: &Folder,
    local: &Scan,
    shown: impl Fn(&Path) -> String,
    updated: &BTreeMap<String, String>,
) -> Scan {
    let root = Path::new(&project.path);
    let mut workflows: Vec<Workflow> = folder
        .docs
        .iter()
        .map(|(_, d)| d.workflow.clone())
        .collect();
    workflows.sort_by(|a, b| a.id.cmp(&b.id));
    for w in &mut workflows {
        let state = |path: &str| {
            let file = root.join(path);
            std::fs::read_to_string(&file).ok()
        };
        for s in &mut w.state_files {
            let text = state(&s.path);
            s.exists = text.is_some();
            s.says = text.map(|t| crate::workflow::says_of(&t));
        }
        if let Some(list) = w.prompt_loop.as_mut().and_then(|p| p.list.as_mut()) {
            let text = state(&list.path);
            list.exists = text.is_some();
            list.says = text.map(|t| crate::workflow::says_of(&t));
        }
    }
    let files = folder
        .docs
        .iter()
        .map(|(p, d)| FileInfo {
            id: d.workflow.id.clone(),
            path: shown(p),
            local: local.workflows.iter().any(|w| w.id == d.workflow.id),
            in_repo: p.starts_with(root),
            updated: updated
                .get(&d.workflow.id)
                .filter(|u| !u.is_empty())
                .cloned(),
        })
        .collect();
    let unreadable = folder
        .unreadable
        .iter()
        .map(|(p, why)| Unreadable {
            path: shown(p),
            why: why.clone(),
        })
        .collect();
    Scan {
        workflows,
        files,
        unreadable,
        ..local.clone()
    }
}

// ---------- the schema ----------

/// The JSON schema of a file (`schemas/escouade-workflow.schema.json`).
#[cfg_attr(not(test), allow(dead_code))]
pub fn schema() -> Value {
    let texts = json!({ "type": "array", "items": { "type": "string" } });
    let agent = json!({
        "type": "object",
        "properties": { "name": { "type": "string" }, "path": { "type": ["string", "null"] }, "description": { "type": "string" } },
        "required": ["name"],
    });
    let step = json!({
        "type": "object",
        "description": "A step: the command that enters it, the sub-agents it launches, the files it reads and writes.",
        "properties": {
            "name": { "type": "string" }, "command": { "type": "string" }, "order": { "type": "integer" },
            "description": { "type": "string" }, "path": { "type": "string" }, "agents": { "type": "array", "items": agent },
            "reads": texts, "writes": texts, "guessed": { "type": "boolean" }, "also": texts,
        },
        "required": ["name", "command", "order", "description", "path", "agents", "reads", "writes"],
    });
    let transition = json!({
        "type": "object",
        "properties": {
            "from": { "type": "string" }, "to": { "type": "string" }, "back": { "type": "boolean" },
            "condition": { "type": ["string", "null"] }, "max": { "type": ["integer", "null"] },
        },
        "required": ["from", "to", "back"],
    });
    let signal = |k: &str| json!({ "type": "object", "properties": { k: { "type": "string" } }, "required": [k], "additionalProperties": false });
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": SCHEMA_URL,
        "title": "A workflow of an Escouade project",
        "description": "A workflow Escouade read in the project, shared with its team in .escouade/workflows/: its definition as read, and the user's corrections apart. Escouade writes it; commit it with your work, Escouade never does.",
        "type": "object",
        "properties": {
            "version": { "type": "integer", "minimum": 1, "description": format!("The version of this file's format ({VERSION} today).") },
            "workflow": {
                "type": "object",
                "description": "The workflow as the reading found it: Escouade writes it again when a reading finds it changed.",
                "properties": {
                    "id": { "type": "string" }, "name": { "type": "string" },
                    "kind": { "enum": ["cycle", "prompt"] }, "harness": { "type": ["string", "null"] },
                    "repo": { "type": "string" }, "doors": { "type": "array", "items": { "type": "object" } },
                    "steps": { "type": "array", "items": step }, "transitions": { "type": "array", "items": transition },
                    "stateFiles": { "type": "array", "items": { "type": "object" } },
                    "exit": { "type": ["string", "null"] }, "promptLoop": { "type": ["object", "null"] },
                    "refined": { "type": "string", "description": "Set by « Affiner… »: the fingerprint of the reading the agent refined; a reading that finds the same leaves the definition as refined." },
                },
                "required": ["id", "name", "kind", "repo", "doors", "steps", "transitions", "stateFiles"],
            },
            "fixes": {
                "type": "object",
                "description": "The user's corrections (Settings › Projects › Workflow): they go before the reading.",
                "properties": {
                    "dismissed": { "type": "boolean", "description": "Not a workflow: kept so that no reading makes it again." },
                    "steps": { "type": "array", "description": "The steps in their order; absent: those read.", "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "from": { "description": "The steps read it stands for (several once merged); none for a step added by hand.", "type": "array", "items": { "type": "string" } },
                            "signals": { "description": "What tells a step added by hand.", "type": "array", "items": { "oneOf": [signal("command"), signal("agent"), signal("file")] } },
                        },
                        "additionalProperties": false,
                    } },
                    "removed": { "description": "The steps read taken out.", "type": "array", "items": { "type": "string" } },
                },
                "additionalProperties": false,
            },
        },
        "required": ["version", "workflow"],
    })
}
