//! The files joined to a message by their path (ESC-95): what the API does not read in the
//! message (any other format, or a file too big for it) is given to Claude as a path, which it
//! reads with its tools. The file is a copy under the data folder (`attachments/<agent>/<uniq>/<name>`,
//! readable without a question: `--add-dir`) or the user's own file, left where it is, whose
//! reading Escouade allows by itself (`AgentMeta::granted_files`). That right is given for a path
//! only the host itself read (`fileorigin`): the window names such a file by an id, never by a path
//! (`Core::attach_origin`), and a message gives a file as the user's own only if it was joined so
//! (`OriginFiles`).

use crate::accounts;
use crate::core::Core;
use crate::engine::Program;
use crate::i18n::Lang;
use crate::paths::{self, DataDir};
use anyhow::{anyhow, bail, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

// Whether two paths are the same file, as an agent keeps its granted files: in `escouade-model` (docs/CRATES.md).
pub use escouade_model::attachfiles::same_file;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::Runtime;

const MB: u64 = 1 << 20;
/// The most a chunk of a copy may weigh (the window cuts its files in chunks of 4 MB at most): a
/// copy is taken up to one chunk past the size it announced, and refused at its end when it is not
/// exactly that.
pub const MAX_CHUNK: u64 = 4 << 20;
/// The owner of the files joined in the form of a ticket, which has no agent yet.
pub(crate) const DRAFT: &str = "draft";
/// The mark of a copy under way, beside its folder (`<uniq>.part`): a copy is no longer under way
/// when its window forgot it (a reload) and the mark is still there, so its half file is found.
const PART: &str = ".part";
/// How long the files of the drafts are kept, whatever `attachKeepDays` says.
const DRAFTS_KEPT: Duration = Duration::from_secs(24 * 3600);

/// A file joined to a message: what the window is told, and tells again when it sends the message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AttachedFile {
    /// Its name, as shown and as Claude is told it.
    pub name: String,
    /// Its path on this machine: the copy, or the user's own file.
    pub path: String,
    /// The same path as the agent's Claude Code reads it (a Linux one in WSL); empty for a draft's
    /// file, whose agent is not known yet. The window's is never believed: it is worked out again
    /// when the message is sent.
    pub claude_path: String,
    /// Its size, in bytes.
    pub size: u64,
    /// The user's own file, nothing copied.
    pub origin: bool,
}

/// The most characters of a name kept, its extension included.
const NAME_CHARS: usize = 150;
/// The most bytes of a name kept: file systems count them (255 on Linux and macOS), and a name of
/// 150 letters of three bytes is more.
const NAME_BYTES: usize = 240;
/// What a file is called when its name leaves nothing.
const NO_NAME: &str = "fichier";

/// A control, or the line and paragraph separators of Unicode (which `is_control` does not count):
/// in a name or a path they would make a second line of the message.
fn breaks_line(c: char) -> bool {
    c.is_control() || matches!(c, '\u{2028}' | '\u{2029}')
}

/// A character a name may not hold: the ones Windows reserves (`<>:"/\|?*`), the controls (a line
/// break would make a second line of the message), and the marks that write the rest of a name
/// backwards (U+202E before `fdp.exe` shows `exe.pdf`: a program passes for a document).
fn reserved(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        || breaks_line(c)
        || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Windows opens these as devices, whatever their extension (`con.png` is the console).
fn is_device(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    let stem = stem.to_lowercase();
    if matches!(
        stem.as_str(),
        "con" | "prn" | "aux" | "nul" | "conin$" | "conout$"
    ) {
        return true;
    }
    let mut chars = stem.chars();
    let (a, b, c, n) = (chars.next(), chars.next(), chars.next(), chars.next());
    chars.next().is_none()
        && matches!(
            (a, b, c),
            (Some('c'), Some('o'), Some('m')) | (Some('l'), Some('p'), Some('t'))
        )
        && matches!(n, Some('1'..='9' | '¹' | '²' | '³'))
}

fn fits(name: &str) -> bool {
    name.chars().count() <= NAME_CHARS && name.len() <= NAME_BYTES
}

/// `name` cut to what a name may be, in its stem: its extension (a dot and 20 letters at most)
/// stays.
fn shortened(name: &str) -> String {
    if fits(name) {
        return name.to_string();
    }
    let (stem, extension) = match name.rfind('.') {
        Some(at) if at > 0 && name.len() - at <= 21 => name.split_at(at),
        _ => (name, ""),
    };
    let mut stem: Vec<char> = stem.chars().collect();
    while !stem.is_empty() && !fits(&format!("{}{extension}", stem.iter().collect::<String>())) {
        stem.pop();
    }
    format!("{}{extension}", stem.into_iter().collect::<String>())
}

/// What a file of a message is called in the folder it is copied to: the name the user knows it by,
/// made safe to be a file name on any system and to never lead out of its folder. No separator or
/// character Windows reserves, no control, no dot or space at its end (Windows drops them, so `..`
/// would be a name that is no name), not the name of a device (`con`, `nul`, `com1`…: prefixed
/// with `_`), 150 characters at most (240 bytes) with its extension kept; `fichier` when nothing is
/// left.
pub fn safe_name(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| if reserved(c) { '_' } else { c })
        .collect();
    let trimmed = replaced.trim_start().trim_end_matches([' ', '.']);
    if trimmed.is_empty() {
        return NO_NAME.into();
    }
    let named = if is_device(trimmed) {
        format!("_{trimmed}")
    } else {
        trimmed.to_string()
    };
    let short = shortened(&named);
    let short = short.trim_end_matches([' ', '.']);
    if short.is_empty() {
        NO_NAME.into()
    } else {
        short.to_string()
    }
}

/// `text` on one line: a control (a line break) is a space.
pub(crate) fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if breaks_line(c) { ' ' } else { c })
        .collect()
}

/// The lines that tell Claude of the files: « Fichier joint : <name>, <path> », one each, written
/// in the language of the texts for Claude (`lang`). Empty without a file.
pub fn message_lines(lang: Lang, files: &[AttachedFile]) -> String {
    files
        .iter()
        .map(|f| {
            let path = if f.claude_path.is_empty() {
                &f.path
            } else {
                &f.claude_path
            };
            tr_in!(
                lang,
                "Fichier joint : {name}, {path}",
                "Attached file: {name}, {path}",
                name = one_line(&f.name),
                path = one_line(path)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` followed by an empty line and the lines of `files`; their lines alone when there is no
/// text.
pub fn message_with_files(lang: Lang, text: &str, files: &[AttachedFile]) -> String {
    let lines = message_lines(lang, files);
    if lines.is_empty() {
        text.to_string()
    } else if text.trim().is_empty() {
        lines
    } else {
        format!("{}\n\n{lines}", text.trim_end())
    }
}

/// Whether the request `tool` with `input` is the reading of exactly one of the files the user
/// joined by their own path (`granted`): the only request Escouade answers without asking.
pub fn granted(granted: &[String], tool: &str, input: &Value) -> bool {
    tool == "Read"
        && input["file_path"]
            .as_str()
            .is_some_and(|path| granted.iter().any(|g| same_file(g, path)))
}

/// The path the agent's `claude` (`program`, none found: the system's) reads `file` by: its Linux
/// path in WSL, a path of Windows that WSL does not reach as it is.
pub fn claude_path_of(program: Option<&Program>, file: &Path) -> String {
    match program {
        Some(program) => program.file_path(file),
        None => file.to_string_lossy().into_owned(),
    }
}

/// `path` as a copy of `folder` (an owner's: `<uniq>/<name>` in it) known by its real place: links
/// resolved, still inside the folder, a file at the depth a copy has. None for anything else (a
/// file of another owner, one the user owns, a link that leaves the folder, a way up with `..`).
pub(crate) fn copy_in(folder: &Path, path: &Path) -> Option<PathBuf> {
    let root = std::fs::canonicalize(folder).ok()?;
    let real = std::fs::canonicalize(path).ok()?;
    let depth = real.strip_prefix(&root).ok()?.components().count();
    (depth == 2 && real.is_file()).then_some(real)
}

/// How many files of the user's own are remembered as joined: the oldest leave first. A message
/// is sent within moments of its files being joined, so only a window that never sends leaves some.
const MAX_JOINED: usize = 1024;

/// The files of the user's own that were joined by their path (`Core::attach_origin`), each with the
/// agent it was joined for. A message gives such a file as joined by its path only if it is in
/// here, for that agent: a window that makes a path up (a file nobody joined, one of another agent)
/// gets nothing read, nothing granted. Not used up by a message: one that fails is sent again.
#[derive(Default)]
pub struct OriginFiles {
    /// (owner, path), oldest first.
    joined: Mutex<VecDeque<(String, String)>>,
}

impl OriginFiles {
    /// `path`, as the window was told it, was joined for `owner`; the newest, a second time too.
    pub(crate) fn add(&self, owner: &str, path: &str) {
        let mut joined = self.joined.lock();
        joined.retain(|(o, p)| !(o == owner && p == path));
        joined.push_back((owner.to_string(), path.to_string()));
        while joined.len() > MAX_JOINED {
            joined.pop_front();
        }
    }

    /// Whether `path` was joined for `owner`, spelt exactly as the window was told it.
    pub(crate) fn has(&self, owner: &str, path: &str) -> bool {
        self.joined
            .lock()
            .iter()
            .any(|(o, p)| o == owner && p == path)
    }
}

/// A random name for the folder of one copy: 8 hexadecimal characters.
pub(crate) fn uniq() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..8].to_string()
}

fn no_copy() -> anyhow::Error {
    anyhow!(tr!(
        "Cette copie n’est plus en cours.",
        "This copy is no longer under way."
    ))
}

fn too_big(name: &str, cap: u64) -> anyhow::Error {
    anyhow!(tr!(
        "{name} dépasse {max}",
        "{name} is over {max}",
        max = tr!("{n} Mo", "{n} MB", n = cap / MB)
    ))
}

/// A copy being written: its file, open, and what the window said of it.
struct Upload {
    owner: String,
    name: String,
    /// `<owner folder>/<uniq>`, and the mark beside it that says it is under way.
    dir: PathBuf,
    marker: PathBuf,
    path: PathBuf,
    size: u64,
    written: u64,
    file: Option<std::fs::File>,
}

impl Upload {
    /// Closes the file and removes what was written of it, its folder and its mark.
    fn discard(&mut self) {
        // Closed first: Windows does not remove an open file.
        self.file = None;
        let _ = std::fs::remove_dir_all(&self.dir);
        let _ = std::fs::remove_file(&self.marker);
    }
}

/// The copies under way, by handle: the window opens one, sends its chunks, finishes it (or
/// cancels). A handle lost with its window leaves a file the purge finds by its mark (`maintain`).
///
/// A copy is in the list from the moment its mark is made to the moment it is done (its mark
/// removed), and the list is held while a mark is made and while the purge looks at the folders:
/// the purge never takes a copy under way for a lost one.
#[derive(Default)]
pub struct Uploads {
    open: Mutex<HashMap<String, Slot>>,
}

/// A copy under way: who it is for and where it is, known without waiting for a write to end.
struct Slot {
    owner: String,
    dir: PathBuf,
    upload: Arc<Mutex<Upload>>,
}

impl Uploads {
    /// The copy `make` starts, under a handle of its own; the list is held while it does.
    fn add_with(&self, make: impl FnOnce() -> Result<Upload>) -> Result<String> {
        let mut open = self.open.lock();
        let upload = make()?;
        let handle = uuid::Uuid::new_v4().simple().to_string();
        let slot = Slot {
            owner: upload.owner.clone(),
            dir: upload.dir.clone(),
            upload: Arc::new(Mutex::new(upload)),
        };
        open.insert(handle.clone(), slot);
        Ok(handle)
    }

    fn get(&self, handle: &str) -> Option<Arc<Mutex<Upload>>> {
        self.open.lock().get(handle).map(|s| s.upload.clone())
    }

    /// The copy is over (done or given up): no longer in the list. Never called with the copy's own
    /// lock held: the list is locked first everywhere else.
    fn forget(&self, handle: &str) {
        self.open.lock().remove(handle);
    }

    /// The copies of `owner`, which goes: no longer under way.
    fn take_owner(&self, owner: &str) -> Vec<Arc<Mutex<Upload>>> {
        let mut open = self.open.lock();
        let handles: Vec<String> = open
            .iter()
            .filter(|(_, s)| s.owner == owner)
            .map(|(h, _)| h.clone())
            .collect();
        handles
            .iter()
            .filter_map(|h| open.remove(h).map(|s| s.upload))
            .collect()
    }

    /// What `f` makes of the folders of the copies under way, which stay under way meanwhile.
    fn with_dirs<T>(&self, f: impl FnOnce(&HashSet<PathBuf>) -> T) -> T {
        let open = self.open.lock();
        let dirs = open.values().map(|s| s.dir.clone()).collect();
        f(&dirs)
    }
}

impl<R: Runtime> Core<R> {
    /// The folder the files joined for `owner` go to: an agent's (it must be there), or the
    /// drafts'. Not made here.
    fn owner_folder(&self, owner: &str) -> Result<PathBuf> {
        if owner == DRAFT {
            return Ok(self.data.drafts_files());
        }
        if !paths::is_agent_folder_id(owner) {
            bail!(tr!(
                "Propriétaire de fichier inconnu : {owner}",
                "Unknown owner of files: {owner}"
            ));
        }
        self.agent(owner)?;
        Ok(self.data.agent_files(owner))
    }

    /// « Taille maximale d’un fichier joint », in bytes; None: no limit.
    pub(crate) fn attach_cap(&self) -> Option<u64> {
        let mb = self.settings.read().attach_max_mb;
        (mb > 0).then(|| u64::from(mb) * MB)
    }

    /// The `claude` the agent's interface or process runs: the interface's while it runs, else the
    /// account's. None when none is found.
    pub(crate) fn program_of(&self, id: &str) -> Option<Program> {
        let h = self.agent(id).ok()?;
        let (running, account) = {
            let rt = h.lock();
            (
                rt.tui.as_ref().map(|t| t.program.clone()),
                rt.meta.account.clone(),
            )
        };
        if running.is_some() {
            return running;
        }
        // Cloned: finding a `claude` may ask WSL, which no lock of the settings waits for.
        let settings = self.settings.read().clone();
        accounts::program(&accounts::get(&settings, &account), &settings)
    }

    /// The path the agent's `claude` reads `file` by (`claude_path_of`); empty for the drafts'
    /// files, whose agent is not known.
    fn claude_path_for(&self, owner: &str, file: &Path) -> String {
        if owner == DRAFT {
            return String::new();
        }
        claude_path_of(self.program_of(owner).as_ref(), file)
    }

    /// A file of `owner` to copy: its name (made safe) and its size are known. Refused when it is
    /// over the cap of the settings. Gives its handle, which the chunks and the end name.
    pub fn attach_open(&self, owner: &str, name: &str, size: u64) -> Result<String> {
        let folder = self.owner_folder(owner)?;
        let name = safe_name(name);
        if let Some(cap) = self.attach_cap().filter(|cap| size > *cap) {
            return Err(too_big(&name, cap));
        }
        self.uploads.add_with(|| {
            let uniq = uniq();
            let dir = folder.join(&uniq);
            let marker = folder.join(format!("{uniq}{PART}"));
            let path = dir.join(&name);
            // Its mark first: a copy that is lost half way is found by it.
            let opened = std::fs::create_dir_all(&folder)
                .and_then(|()| std::fs::write(&marker, b""))
                .and_then(|()| std::fs::create_dir(&dir))
                .and_then(|()| {
                    std::fs::File::options()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                });
            match opened {
                Ok(file) => Ok(Upload {
                    owner: owner.to_string(),
                    name,
                    dir,
                    marker,
                    path,
                    size,
                    written: 0,
                    file: Some(file),
                }),
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&dir);
                    let _ = std::fs::remove_file(&marker);
                    Err(anyhow!(tr!(
                        "{name} n’a pas pu être copié : {e}",
                        "{name} couldn’t be copied: {e}"
                    )))
                }
            }
        })
    }

    /// The next bytes of the copy `handle`. Refused past one chunk beyond the size announced, and
    /// past the cap of the settings (read on what is written, not on what was said).
    pub fn attach_chunk(&self, handle: &str, bytes: &[u8]) -> Result<()> {
        let slot = self.uploads.get(handle).ok_or_else(no_copy)?;
        let mut up = slot.lock();
        let total = up.written + bytes.len() as u64;
        if let Some(cap) = self.attach_cap().filter(|cap| total > *cap) {
            return Err(too_big(&up.name, cap));
        }
        if total > up.size.saturating_add(MAX_CHUNK) {
            bail!(tr!(
                "{name} est plus gros que ce qui était annoncé.",
                "{name} is bigger than announced.",
                name = up.name
            ));
        }
        let name = up.name.clone();
        let Some(file) = up.file.as_mut() else {
            return Err(no_copy());
        };
        file.write_all(bytes).map_err(|e| {
            anyhow!(tr!(
                "{name} n’a pas pu être copié : {e}",
                "{name} couldn’t be copied: {e}"
            ))
        })?;
        up.written = total;
        Ok(())
    }

    /// The end of the copy `handle`: what was written is the size announced, and the file is done.
    /// Anything else, and nothing is left of it.
    pub fn attach_finish(&self, handle: &str) -> Result<AttachedFile> {
        let slot = self.uploads.get(handle).ok_or_else(no_copy)?;
        // Still in the list while it is closed and its mark removed: the purge leaves it alone.
        let done = Self::finish_copy(&mut slot.lock());
        self.uploads.forget(handle);
        let (owner, name, path, size) = done?;
        Ok(AttachedFile {
            name,
            claude_path: self.claude_path_for(&owner, &path),
            path: path.to_string_lossy().into_owned(),
            size,
            origin: false,
        })
    }

    /// `attach_finish`, up to the copy itself: its owner, its name, its path and its size, or its
    /// refusal, nothing left of it.
    fn finish_copy(up: &mut Upload) -> Result<(String, String, PathBuf, u64)> {
        let Some(file) = up.file.take() else {
            return Err(no_copy());
        };
        // Closed: Windows does not move or remove an open file.
        drop(file);
        let (name, written, size) = (up.name.clone(), up.written, up.size);
        if written != size {
            up.discard();
            bail!(tr!(
                "{name} n’a pas été copié en entier ({written} octets sur {size}).",
                "{name} wasn’t copied whole ({written} of {size} bytes)."
            ));
        }
        // Done: no longer under way. A mark that stays would have its file removed as a lost one.
        if let Err(e) = std::fs::remove_file(&up.marker) {
            up.discard();
            bail!(tr!(
                "{name} n’a pas pu être copié : {e}",
                "{name} couldn’t be copied: {e}"
            ));
        }
        Ok((up.owner.clone(), name, up.path.clone(), size))
    }

    /// The copy `handle` is given up (a failure, its chip removed while it was copied): what was
    /// written goes. A handle that is gone (finished, lost) is no error.
    pub fn attach_cancel(&self, handle: &str) {
        if let Some(slot) = self.uploads.get(handle) {
            slot.lock().discard();
            self.uploads.forget(handle);
        }
    }

    /// A finished copy `path` of `owner` removed (its chip removed), with its folder. Refused for
    /// any path that is not a copy of that owner's own folder: another owner's, a file of the
    /// user's, a link that leaves it. A copy already gone is nothing to remove.
    pub fn attach_discard(&self, owner: &str, path: &str) -> Result<()> {
        let folder = self.owner_folder(owner)?;
        let given = Path::new(path);
        let Some(real) = copy_in(&folder, given) else {
            // Gone already (the purge, a second click): only for a path that is lexically one of
            // the folder's, never one with `..` in it.
            let gone = std::fs::symlink_metadata(given)
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
            let plain = given
                .components()
                .all(|c| !matches!(c, Component::ParentDir));
            if gone && plain && given.starts_with(&folder) {
                return Ok(());
            }
            return Err(paths::outside(path));
        };
        std::fs::remove_file(&real)?;
        if let Some(uniq) = real.parent() {
            // Empty now: only its own folder goes, and a mark of a copy under way would stay.
            let _ = std::fs::remove_dir(uniq);
        }
        Ok(())
    }

    /// What there is to know of the file of the user's own at `path` for it to be joined as it is:
    /// its name, made safe, and its size. Refused when it is no absolute path, when a control
    /// character is in it (a line break would make a second line of the message), when it is in
    /// Escouade's own data folder (its state, its settings, the copies of every agent: reading
    /// those is granted to no one), when it is no ordinary file or cannot be read, and when it is
    /// over the cap of the settings.
    fn check_origin(&self, path: &str) -> Result<(String, u64)> {
        let file = Path::new(path);
        let label = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string());
        let name = safe_name(&label);
        if path.chars().any(breaks_line) || !file.is_absolute() {
            bail!(tr!(
                "Le chemin de {name} n’est pas utilisable.",
                "The path of {name} can’t be used."
            ));
        }
        let unreadable = |e: std::io::Error| {
            anyhow!(tr!(
                "{name} n’a pas pu être lu : {e}",
                "{name} couldn’t be read: {e}"
            ))
        };
        let meta = std::fs::metadata(file).map_err(unreadable)?;
        // Where it really is, links resolved on both sides (a link in a folder of the user's that
        // leads in is the file it leads to); the folder itself is no file to join either.
        let real = std::fs::canonicalize(file).map_err(unreadable)?;
        let root = std::fs::canonicalize(self.data.root())
            .unwrap_or_else(|_| self.data.root().to_path_buf());
        if real.starts_with(&root) {
            bail!(tr!(
                "Un fichier du dossier de données d’Escouade ne se joint pas par son chemin.",
                "A file in Escouade’s data folder can’t be joined by its path."
            ));
        }
        if !meta.is_file() {
            bail!(tr!(
                "{name} n’est pas un fichier ordinaire.",
                "{name} isn’t a regular file."
            ));
        }
        // Read at least once: a file a program holds, or the system keeps from the user, is no
        // file Claude Code will read either.
        std::fs::File::open(file).map_err(unreadable)?;
        if let Some(cap) = self.attach_cap().filter(|cap| meta.len() > *cap) {
            return Err(too_big(&name, cap));
        }
        Ok((name, meta.len()))
    }

    /// The file of the user's own that the host read the path of, joined for `owner` as it is:
    /// checked, and nothing copied (`origin`). `id` is the one the page was answered a path for
    /// (`file_origin`), used up here: the window never gives a path, so it cannot name a file the
    /// host did not read. The file is remembered as joined for `owner` (`OriginFiles`), and the
    /// agent's reading of it is allowed once a message that has it is sent.
    pub fn attach_origin(&self, owner: &str, id: &str) -> Result<AttachedFile> {
        // The owner first: a mistake in it does not use the id up.
        self.owner_folder(owner)?;
        let Some(path) = self.file_origins.take_handed(id) else {
            bail!(tr!(
                "Escouade ne connaît pas ce fichier par son chemin (identifiant inconnu ou déjà utilisé).",
                "Escouade doesn’t know this file by its path (unknown or already used id)."
            ));
        };
        let (name, size) = self.check_origin(&path)?;
        self.origin_files.add(owner, &path);
        Ok(AttachedFile {
            name,
            claude_path: self.claude_path_for(owner, Path::new(&path)),
            path,
            size,
            origin: true,
        })
    }

    /// The files of a message to the agent `id`, as the window sent them, checked again: a copy is
    /// one of the agent's own folder (`copy_in`), a file of the user's own was joined to this agent
    /// by its path (`OriginFiles`: a path the window makes up is refused) and is there and fit to
    /// be joined (`check_origin`). Their names, sizes and the path Claude reads them by are worked
    /// out here, never believed. One that fails refuses them all. Nothing is used up: the message
    /// that fails is sent again with the same files.
    pub(crate) fn checked_files(
        &self,
        id: &str,
        files: Vec<AttachedFile>,
    ) -> Result<Vec<AttachedFile>> {
        if files.is_empty() {
            return Ok(files);
        }
        let folder = self.data.agent_files(id);
        let program = self.program_of(id);
        files
            .into_iter()
            .map(|f| {
                let (name, size) = if f.origin {
                    if !self.origin_files.has(id, &f.path) {
                        bail!(tr!(
                            "« {name} » n’a pas été joint par son chemin d’origine.",
                            "“{name}” wasn’t joined by its own path.",
                            name = one_line(&f.name)
                        ));
                    }
                    self.check_origin(&f.path)?
                } else {
                    let real = copy_in(&folder, Path::new(&f.path)).ok_or_else(|| {
                        anyhow!(tr!(
                            "« {name} » n’est pas un fichier joint de cet agent.",
                            "“{name}” isn’t a file joined to this agent.",
                            name = one_line(&f.name)
                        ))
                    })?;
                    let size = std::fs::metadata(&real)?.len();
                    let name = real
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    (name, size)
                };
                Ok(AttachedFile {
                    name,
                    claude_path: claude_path_of(program.as_ref(), Path::new(&f.path)),
                    path: f.path,
                    size,
                    origin: f.origin,
                })
            })
            .collect()
    }

    /// The agent `id` may read the files of the user's own that were just joined (`files`, as
    /// `checked_files` gave them), the next time Claude Code asks (`granted`).
    pub(crate) fn grant_files(&self, id: &str, files: &[AttachedFile]) {
        let own: Vec<&AttachedFile> = files.iter().filter(|f| f.origin).collect();
        if own.is_empty() {
            return;
        }
        let Ok(h) = self.agent(id) else { return };
        {
            let mut rt = h.lock();
            for f in own {
                rt.meta.grant_file(f.claude_path.clone());
            }
        }
        self.request_save();
    }

    /// The files joined to the agent `id` (copies only) go with it, and the copies it was still
    /// making.
    pub(crate) fn forget_attachments(&self, id: &str) {
        for slot in self.uploads.take_owner(id) {
            slot.lock().discard();
        }
        remove_agent_files(&self.data, id);
    }

    /// What `maintain` does, for the copies of this app: the ones under way are not touched.
    pub(crate) fn maintain_attachments(&self) {
        let keep_days = self.settings.read().attach_keep_days;
        let root = self.data.attachments();
        self.uploads
            .with_dirs(|open| maintain(&root, keep_days, open));
        // The tickets' files are in their own folders, under the tickets' (`ticketfiles`).
        crate::ticketfiles::maintain(&self.data.tickets(), keep_days);
    }

    /// The folder of the agent's joined files, which it reads without a question (`--add-dir` for
    /// Claude Code), made here first. None when it cannot be made (the agent starts, with that
    /// folder asked for).
    pub(crate) fn agent_read_dir(&self, id: &str) -> Option<std::path::PathBuf> {
        let dir = self.data.agent_files(id);
        match std::fs::create_dir_all(&dir) {
            Ok(()) => Some(dir),
            Err(e) => {
                log::warn!("agent {id}: {} not made: {e}", dir.display());
                None
            }
        }
    }
}

/// The folder of the files joined to the agent `id`, removed with its files, and the configuration
/// folder of its harness (`DataDir::agent_home`). Never the folder of all the owners that have no
/// name of their own (`DataDir::agent_files`).
pub(crate) fn remove_agent_files(data: &DataDir, id: &str) {
    if !paths::is_agent_folder_id(id) {
        return;
    }
    for dir in [data.agent_files(id), data.agent_home(id)] {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("{} not removed: {e}", dir.display());
            }
        }
    }
}

/// Tidies the files joined to messages in `root` (`<owner>/<uniq>/<name>`): the copies lost half
/// written go (the mark `<uniq>.part` is still there and no window under way, `open`, writes them),
/// the drafts' files a day old go, and, when `keep_days` is more than 0, any copy older than that.
/// Their folders go when they are empty, never an owner's. No file of the user's own is in there.
pub fn maintain(root: &Path, keep_days: u32, open: &HashSet<PathBuf>) {
    let Ok(owners) = std::fs::read_dir(root) else {
        return;
    };
    for owner in owners.flatten() {
        if !owner.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let limit = if owner.file_name() == "_drafts" {
            Some(DRAFTS_KEPT)
        } else {
            (keep_days > 0).then(|| Duration::from_secs(u64::from(keep_days) * 24 * 3600))
        };
        tidy_owner(&owner.path(), limit, open);
    }
}

/// `maintain`, for the folder of one owner: its copies older than `limit` (none: any age) go.
pub(crate) fn tidy_owner(folder: &Path, limit: Option<Duration>, open: &HashSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let (path, name) = (
            entry.path(),
            entry.file_name().to_string_lossy().into_owned(),
        );
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_file() {
            // A mark: its copy is lost unless it is under way.
            if let Some(uniq) = name.strip_suffix(PART) {
                let dir = folder.join(uniq);
                if !open.contains(&dir) {
                    let _ = std::fs::remove_dir_all(&dir);
                    let _ = std::fs::remove_file(&path);
                }
            }
        } else if kind.is_dir() && !open.contains(&path) {
            if let Some(limit) = limit {
                purge_copy(&path, limit);
            }
        }
    }
}

/// The files of the copy's folder `dir` older than `limit`, and the folder once it is empty.
fn purge_copy(dir: &Path, limit: Duration) {
    let Ok(files) = std::fs::read_dir(dir) else {
        return;
    };
    for file in files.flatten() {
        let aged = file
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|age| age > limit);
        if aged && file.file_type().is_ok_and(|t| !t.is_dir()) {
            let _ = std::fs::remove_file(file.path());
        }
    }
    // Only an empty folder goes.
    let _ = std::fs::remove_dir(dir);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_name_the_user_knows_stays_as_it_is() {
        for name in [
            "rapport.xlsx",
            "résumé été.pdf",
            ".gitignore",
            "a b (1).tar.gz",
        ] {
            assert_eq!(safe_name(name), name);
        }
    }

    #[test]
    fn a_name_never_leads_out_of_its_folder() {
        assert_eq!(safe_name("../../etc/passwd"), ".._.._etc_passwd");
        assert_eq!(safe_name(r"C:\Users\x\a.txt"), "C__Users_x_a.txt");
        assert_eq!(safe_name(r"..\..\boot.ini"), ".._.._boot.ini");
        for name in ["..", ".", "...", "", "   ", " . . "] {
            assert_eq!(safe_name(name), "fichier", "{name:?}");
        }
        for name in ["../../etc/passwd", r"a\b", "a/b", "/abs", r"\\server\share"] {
            let safe = safe_name(name);
            assert!(!safe.contains(['/', '\\']), "{safe}");
        }
    }

    #[test]
    fn the_characters_windows_reserves_and_the_controls_are_replaced() {
        assert_eq!(safe_name("a<b>c:d\"e|f?g*h"), "a_b_c_d_e_f_g_h");
        assert_eq!(safe_name("a\nb\tc\0d\u{7f}e"), "a_b_c_d_e");
        // A line break of the name never makes a second line of the message.
        assert!(!safe_name("a\r\nb").contains(['\r', '\n']));
        // Written backwards by a right-to-left override: « fdp.exe » shown as « exe.pdf ».
        assert_eq!(safe_name("rapport\u{202e}fdp.exe"), "rapport_fdp.exe");
    }

    #[test]
    fn a_name_ends_with_neither_a_dot_nor_a_space() {
        assert_eq!(safe_name("report. . "), "report");
        assert_eq!(safe_name("file.txt."), "file.txt");
        assert_eq!(safe_name("  lead.txt"), "lead.txt");
    }

    #[test]
    fn the_names_of_windows_devices_are_prefixed() {
        for (name, safe) in [
            ("con", "_con"),
            ("NUL.txt", "_NUL.txt"),
            ("com1", "_com1"),
            ("LPT9.log", "_LPT9.log"),
            ("aux.tar.gz", "_aux.tar.gz"),
            ("Prn", "_Prn"),
            ("COM\u{b9}", "_COM\u{b9}"),
            ("con.", "_con"),
            ("CONIN$", "_CONIN$"),
        ] {
            assert_eq!(safe_name(name), safe);
        }
        // Not devices.
        for name in ["com0", "com10", "console.txt", "nullable", "lpt", "a.con"] {
            assert_eq!(safe_name(name), name);
        }
    }

    #[test]
    fn a_long_name_is_cut_in_the_stem_and_keeps_its_extension() {
        let long = format!("{}.xlsx", "a".repeat(300));
        let safe = safe_name(&long);
        assert_eq!(safe.chars().count(), 150);
        assert!(safe.ends_with("aaa.xlsx"), "{safe}");
        // Exactly the limit: untouched.
        let exact = "b".repeat(150);
        assert_eq!(safe_name(&exact), exact);
        // No room kept for something that is no extension.
        let odd = format!("{}.{}", "c".repeat(10), "d".repeat(300));
        assert!(safe_name(&odd).chars().count() <= 150);
        // File systems count bytes (255 at most on Linux and macOS), and a name is what Claude is
        // told: 200 letters of two bytes, or of three, still make a name the system takes.
        for letter in ['é', '文'] {
            let name: String = std::iter::repeat_n(letter, 200).collect();
            let safe = safe_name(&format!("{name}.pdf"));
            assert!(safe.chars().count() <= 150, "{}", safe.chars().count());
            assert!(safe.len() <= 240, "{}", safe.len());
            assert!(safe.ends_with(".pdf"), "{safe}");
        }
    }

    #[test]
    fn a_safe_name_is_its_own_safe_name() {
        for name in [
            "../../x",
            "con",
            "a\nb",
            &"z".repeat(400),
            "a.b.c. ",
            "文".repeat(300).as_str(),
        ] {
            let once = safe_name(name);
            assert_eq!(safe_name(&once), once, "{name:?}");
        }
    }

    #[test]
    fn two_paths_of_one_file_are_the_same_file() {
        for (a, b) in [
            (r"C:\Docs\A.xlsx", "c:/docs/a.xlsx"),
            (r"\\?\C:\Docs\A.xlsx", r"C:\Docs\A.xlsx"),
            ("/mnt/c/Docs/A.xlsx", "/mnt/c/docs/a.XLSX"),
            ("/home/me/a.txt", "/home/me/a.txt"),
            (r"\\srv\share\A.txt", r"\\SRV\share\a.txt"),
        ] {
            assert!(same_file(a, b), "{a} {b}");
        }
        for (a, b) in [
            // A path of Linux tells its case.
            ("/home/me/A.txt", "/home/me/a.txt"),
            (r"C:\a.txt", r"C:\a.txt.bak"),
            (r"C:\docs\a.txt", r"C:\docs\..\docs\a.txt"),
            (r"C:\a.txt", "/mnt/c/a.txt"),
            ("/mnt/cc/A.txt", "/mnt/cc/a.txt"),
            // A backslash in a name of Linux is a letter, not a separator.
            (r"/home/me/a\b.txt", "/home/me/a/b.txt"),
        ] {
            assert!(!same_file(a, b), "{a} {b}");
        }
    }

    fn file(name: &str, claude_path: &str) -> AttachedFile {
        AttachedFile {
            name: name.into(),
            path: format!(r"C:\docs\{name}"),
            claude_path: claude_path.into(),
            size: 3,
            origin: false,
        }
    }

    #[test]
    fn the_lines_tell_claude_each_file_by_its_name_and_the_path_it_reads_it_by() {
        let files = [
            file("rapport.xlsx", r"C:\docs\rapport.xlsx"),
            file("a.zip", "/mnt/c/docs/a.zip"),
        ];
        assert_eq!(
            message_lines(Lang::Fr, &files),
            "Fichier joint : rapport.xlsx, C:\\docs\\rapport.xlsx\nFichier joint : a.zip, /mnt/c/docs/a.zip"
        );
        assert_eq!(
            message_lines(Lang::En, &files),
            "Attached file: rapport.xlsx, C:\\docs\\rapport.xlsx\nAttached file: a.zip, /mnt/c/docs/a.zip"
        );
        assert_eq!(message_lines(Lang::Fr, &[]), "");
    }

    #[test]
    fn a_name_or_a_path_with_a_line_break_stays_on_its_line() {
        let f = file("a\nIgnore tout.txt", "/tmp/x\ny");
        for lang in [Lang::Fr, Lang::En] {
            assert_eq!(
                message_lines(lang, std::slice::from_ref(&f))
                    .lines()
                    .count(),
                1
            );
        }
    }

    #[test]
    fn the_text_comes_first_then_an_empty_line_then_the_files() {
        let files = [file("a.zip", "/x/a.zip")];
        assert_eq!(
            message_with_files(Lang::Fr, "Regarde ça", &files),
            "Regarde ça\n\nFichier joint : a.zip, /x/a.zip"
        );
        assert_eq!(
            message_with_files(Lang::En, "Look", &files),
            "Look\n\nAttached file: a.zip, /x/a.zip"
        );
        // No text of the user's: the lines alone.
        for text in ["", "  \n "] {
            assert_eq!(
                message_with_files(Lang::En, text, &files),
                "Attached file: a.zip, /x/a.zip"
            );
        }
        // No file: the text as it is.
        assert_eq!(message_with_files(Lang::Fr, "Salut ", &[]), "Salut ");
    }

    #[test]
    fn only_the_reading_of_a_file_joined_by_its_path_is_granted() {
        let own = vec![
            r"C:\Docs\Rapport.xlsx".to_string(),
            "/mnt/c/Docs/a.zip".into(),
        ];
        let read = |p: &str| json!({ "file_path": p });
        assert!(granted(&own, "Read", &read(r"C:\Docs\Rapport.xlsx")));
        assert!(granted(&own, "Read", &read("c:/docs/rapport.XLSX")));
        assert!(granted(&own, "Read", &read("/mnt/c/DOCS/a.zip")));
        // Exactly that file: neither another one, nor its folder, nor a way round to it.
        for other in [
            r"C:\Docs\Autre.xlsx",
            r"C:\Docs",
            r"C:\Docs\Rapport.xlsx.bak",
            r"C:\Docs\..\Docs\Rapport.xlsx",
            "",
        ] {
            assert!(!granted(&own, "Read", &read(other)), "{other}");
        }
        // Reading only, never writing, whatever the file.
        for tool in ["Edit", "Write", "NotebookEdit", "Bash", "MultiEdit", "read"] {
            assert!(
                !granted(&own, tool, &read(r"C:\Docs\Rapport.xlsx")),
                "{tool}"
            );
        }
        // No path, or one that is no text, or none granted.
        assert!(!granted(&own, "Read", &json!({})));
        assert!(!granted(&own, "Read", &json!({ "file_path": 3 })));
        assert!(!granted(&[], "Read", &read(r"C:\Docs\Rapport.xlsx")));
    }
}
