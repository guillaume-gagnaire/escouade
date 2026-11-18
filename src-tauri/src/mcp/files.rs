//! The other files of a ticket (not its images) through the server (ESC-108): the ones the tools
//! attach (`create_ticket`, `update_ticket`), the list `get_ticket` gives, the part of one
//! `get_ticket_file` reads back and the copies `split_ticket` hands down. They go where the window's
//! go: a new one is first a copy of the drafts, made as the form makes it (`Core::attach_open`, its
//! chunks, its end), then the core checks the list (`ticketfiles::check`: 20 at most, each no
//! bigger than the size the settings allow) and moves it into the ticket's folder. Nothing here
//! decides those limits; what the tools add is the ways in (a content in base64, or the path of a
//! file), the checks made before a byte is copied, and the care that a name or a path from the
//! model leads nowhere it should not.

use super::act::may_act_on;
use super::images::{open_within, shown, working_folder};
use super::read::own_ticket;
use super::resolve;
use super::tools::{FileArg, FileRef, GetTicketFileArgs, ToolError};
use super::Caller;
use crate::attachfiles::{self, DRAFT};
use crate::core::Core;
use crate::i18n;
use crate::model::{Ticket, TicketFile};
use crate::ticketfiles::{self, MAX_FILES};
use crate::ticketimages;
use crate::tickets::DraftFile;
use base64::Engine;
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The bytes of a file `get_ticket_file` gives unless asked for more, and the most it gives: those
/// of `get_background_output` (Claude Code refuses an answer of more than about 25,000 tokens).
pub(super) const READ_DEFAULT: usize = 8192;
pub(super) const READ_MAX: usize = 65_536;

/// What is read of a file at a time while it is copied into the drafts.
const CHUNK: usize = 1 << 20;

const MB: u64 = 1 << 20;

fn failed(why: String) -> ToolError {
    ToolError::Failed(why)
}

/// `e` as the `n`th file of a list given to a tool: a refusal stays one.
fn in_list(n: usize, e: ToolError) -> ToolError {
    let said = |why: String| {
        tr!(
            "Fichier {n} de la liste : {why}",
            "File {n} of the list: {why}"
        )
    };
    match e {
        ToolError::Failed(why) => ToolError::Failed(said(why)),
        ToolError::Refused(why) => ToolError::Refused(said(why)),
    }
}

/// The name a file is copied by: what follows the last slash of what was given, made safe as the
/// window makes the name of a file it copies (`attachfiles::safe_name`: on one line, no character
/// Windows refuses, cut to a length a disk takes). Only a name that is nothing once cleaned fails.
pub(super) fn name(raw: &str) -> Result<String, ToolError> {
    let last = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    let visible = super::act::visible_line(last);
    if visible.is_empty() {
        return Err(failed(tr!(
            "Le nom du fichier est vide.",
            "The file’s name is empty."
        )));
    }
    Ok(attachfiles::safe_name(&visible))
}

fn size_label(cap: u64) -> String {
    tr!("{n} Mo", "{n} MB", n = cap / MB)
}

fn too_big(name: &str, cap: u64) -> ToolError {
    failed(ticketfiles::too_big(i18n::ui(), name, cap).to_string())
}

fn too_many() -> ToolError {
    failed(ticketfiles::too_many(i18n::ui()))
}

// ---------- the files a ticket has ----------

/// The files of a ticket (`had`) that `refs` name, as ranks in its list, in the order named and
/// each once. A reference is a file or a rank, exactly one of the two, and fits a file of the
/// ticket: any other (a name that leads up, a path, a rank off the list) fails, and nothing is
/// chosen.
pub(super) fn select(had: &[TicketFile], refs: &[FileRef]) -> Result<Vec<usize>, ToolError> {
    let mut chosen = Vec::with_capacity(refs.len());
    for reference in refs {
        let at = find(had, reference)?;
        if !chosen.contains(&at) {
            chosen.push(at);
        }
    }
    Ok(chosen)
}

/// The rank, in `had`, of the file `reference` names.
fn find(had: &[TicketFile], reference: &FileRef) -> Result<usize, ToolError> {
    let hint = tr!(
        "Lis get_ticket pour voir les fichiers du ticket (files : file, index).",
        "Read get_ticket to see the ticket’s files (files: file, index)."
    );
    match (&reference.file, reference.index) {
        // Compared to the files the ticket has, as they are: a name is never turned into a path.
        (Some(file), None) => had.iter().position(|f| f.file == *file).ok_or_else(|| {
            let file = shown(file);
            failed(tr!(
                "Le ticket n’a pas de fichier « {file} ». {hint}",
                "The ticket has no file “{file}”. {hint}"
            ))
        }),
        (None, Some(index)) => match usize::try_from(index) {
            Ok(at) if at < had.len() => Ok(at),
            _ if had.is_empty() => Err(failed(tr!(
                "Le ticket n’a pas de fichier de rang {index} : il n’en a aucun. {hint}",
                "The ticket has no file of rank {index}: it has none. {hint}"
            ))),
            _ => {
                let (count, last) = (had.len(), had.len() - 1);
                Err(failed(tr!(
                    "Le ticket n’a pas de fichier de rang {index} : il en a {count} (rangs 0 à {last}). {hint}",
                    "The ticket has no file of rank {index}: it has {count} (ranks 0 to {last}). {hint}"
                )))
            }
        },
        (Some(_), Some(_)) => Err(failed(tr!(
            "Désigne le fichier par son file ou son rang (index), pas les deux. {hint}",
            "Name the file by its file or its rank (index), not both. {hint}"
        ))),
        (None, None) => Err(failed(tr!(
            "Désigne le fichier par son file ou son rang (index). {hint}",
            "Name the file by its file or its rank (index). {hint}"
        ))),
    }
}

/// The size of a ticket's file; None when it is gone, or is no file of the ticket's folder
/// (`ticketfiles::present`: a state edited by hand could name a file anywhere, never looked at).
fn size_of(dir: &Path, file: &TicketFile) -> Option<u64> {
    let path = ticketfiles::present(dir, &file.file)?;
    std::fs::metadata(path).ok().map(|m| m.len())
}

/// A file of a ticket, as `get_ticket` lists it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileView {
    /// Its rank in the ticket’s list, from 0.
    index: usize,
    file: String,
    name: String,
    /// In bytes; None when its file is gone (the purge of the settings, « Conserver les fichiers »).
    size: Option<u64>,
}

/// The files of a ticket whose folder is `dir`, in the ticket’s order: all of them, those whose
/// file is gone too, which keep their rank.
pub(super) fn listed(dir: &Path, files: &[TicketFile]) -> Vec<FileView> {
    files
        .iter()
        .enumerate()
        .map(|(index, f)| FileView {
            index,
            file: f.file.clone(),
            name: f.name.clone(),
            size: size_of(dir, f),
        })
        .collect()
}

/// A part of a file, as `get_ticket_file` gives it.
#[derive(Debug, PartialEq)]
pub(super) struct Piece {
    /// Where it starts in the file, in bytes.
    pub(super) offset: u64,
    /// "text" (UTF-8, cut at characters) or "base64".
    pub(super) encoding: &'static str,
    pub(super) content: String,
    /// How many bytes of the file it holds.
    pub(super) length: usize,
}

/// What `bytes`, read at `offset` of a file of `size` bytes, is given as: its text when it is text
/// (UTF-8 without a NUL), the characters a cut split at either end left out; else its base64.
pub(super) fn piece(bytes: Vec<u8>, offset: u64, size: u64) -> Piece {
    let end = offset + bytes.len() as u64;
    // A cut at the start may fall in a character: its last bytes are skipped (3 at most).
    let skip = if offset > 0 {
        bytes
            .iter()
            .take(3)
            .take_while(|b| (**b & 0xC0) == 0x80)
            .count()
    } else {
        0
    };
    let rest = &bytes[skip..];
    let text = match std::str::from_utf8(rest) {
        Ok(text) => Some(text),
        // A cut at the end may fall in a character: its first bytes are left out.
        Err(e) if e.error_len().is_none() && end < size && rest.len() - e.valid_up_to() < 4 => {
            std::str::from_utf8(&rest[..e.valid_up_to()]).ok()
        }
        Err(_) => None,
    };
    match text.filter(|t| !t.contains('\0')) {
        Some(text) => Piece {
            offset: offset + skip as u64,
            encoding: "text",
            content: text.to_string(),
            length: text.len(),
        },
        None => Piece {
            offset,
            encoding: "base64",
            length: bytes.len(),
            content: base64::engine::general_purpose::STANDARD.encode(&bytes),
        },
    }
}

/// `max` bytes of the file at `path` from `offset`, or its last `max` bytes without one; with its
/// size.
pub(super) fn read_part(
    path: &Path,
    offset: Option<u64>,
    max: usize,
) -> std::io::Result<(Vec<u8>, u64, u64)> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let start = match offset {
        Some(at) => at.min(size),
        None => size.saturating_sub(max as u64),
    };
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::with_capacity(max.min(size as usize));
    file.take(max as u64).read_to_end(&mut bytes)?;
    Ok((bytes, start, size))
}

/// `get_ticket_file`: a part of a file of the ticket, its end by default, as text or base64, with
/// what it is; and, for Claude outside Escouade, where the file is on this computer, to read it
/// whole by its own means. Not read, and said so, when its file is gone.
pub(super) async fn fetch(
    core: &Core,
    caller: &Caller,
    a: GetTicketFileArgs,
) -> Result<String, ToolError> {
    let t = match a.ticket.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(asked) => resolve::ticket(core, caller, asked)?,
        None => own_ticket(core, caller)?,
    };
    // The sheet of a ticket (`get_ticket`) is open to all, its files are the files of its project:
    // an agent reads those of the projects it acts on.
    may_act_on(core, caller, &t.project_id)?;
    let index = find(
        &t.files,
        &FileRef {
            file: a.file,
            index: a.index,
        },
    )?;
    let f = t.files[index].clone();
    let max = a.bytes.map_or(READ_DEFAULT, |b| {
        usize::try_from(b).unwrap_or(usize::MAX).clamp(1, READ_MAX)
    });
    let gone = || {
        let name = shown(&f.name);
        failed(tr!(
            "Le fichier « {name} » ne peut pas être lu : il a disparu du dossier du ticket ou n’est pas lisible.",
            "The file “{name}” can’t be read: it is gone from the ticket’s folder or unreadable."
        ))
    };
    let (dir, file, offset) = (core.data.ticket_dir(&t.id), f.file.clone(), a.offset);
    let read = tokio::task::spawn_blocking(move || {
        // Read where it really is; given as the agent's first message gives it (a canonical path
        // of Windows starts with `\\?\`, which no one types).
        let real = ticketfiles::present(&dir, &file)?;
        let part = read_part(&real, offset, max).ok()?;
        Some((ticketfiles::path_of(&dir, &file)?, part))
    })
    .await
    .map_err(|e| failed(e.to_string()))?;
    let (path, (bytes, start, size)) = read.ok_or_else(gone)?;
    let part = piece(bytes, start, size);
    let mut says = json!({
        "ticket": t.key,
        "index": index,
        "file": f.file,
        "name": f.name,
        "size": size,
        "offset": part.offset,
        "length": part.length,
        "encoding": part.encoding,
        "content": part.content,
    });
    // Claude outside Escouade reads it by its own means, whole; an agent of Escouade has the path
    // of its ticket's files in its first message, and reads no other ticket's by a path.
    if matches!(caller, Caller::External) {
        says["path"] = json!(path.to_string_lossy());
    }
    Ok(says.to_string())
}

// ---------- the files a tool gives ----------

/// The copies of the drafts a call made: those the core did not move into a ticket (a refusal, a
/// failure, the call stopped half way) go once it is over, as the form's chip removed does.
pub(super) struct Drafted {
    core: Arc<Core>,
    paths: Vec<String>,
}

impl Drafted {
    pub(super) fn new(core: &Arc<Core>) -> Self {
        Self {
            core: core.clone(),
            paths: Vec::new(),
        }
    }
}

impl Drop for Drafted {
    fn drop(&mut self) {
        for path in &self.paths {
            // One moved into a ticket is gone: nothing to remove.
            if self.core.attach_discard(DRAFT, path).is_err() {
                log::warn!("a copy of the drafts made by a tool was not removed");
            }
        }
    }
}

/// What `from` holds, `length` bytes, written as a copy of the drafts named `name`, as the form
/// makes one (`Core::attach_open`, its chunks, its end: the size of the settings is the core's to
/// check, on what is written). Gives its path; nothing is left of it when it fails.
fn to_drafts(core: &Core, name: &str, from: impl Read, length: u64) -> Result<String, ToolError> {
    let handle = core
        .attach_open(DRAFT, name, length)
        .map_err(|e| failed(e.to_string()))?;
    let copied = (|| -> anyhow::Result<()> {
        // A file that grew since its length was read is cut there.
        let mut from = from.take(length);
        let mut buf = vec![0u8; CHUNK];
        loop {
            let n = from.read(&mut buf)?;
            if n == 0 {
                return Ok(());
            }
            core.attach_chunk(&handle, &buf[..n])?;
        }
    })();
    if let Err(e) = copied {
        core.attach_cancel(&handle);
        return Err(failed(tr!(
            "{name} n’a pas pu être copié : {e}",
            "{name} couldn’t be copied: {e}"
        )));
    }
    // A file that shrank since is refused there: it was not copied whole.
    core.attach_finish(&handle)
        .map(|f| f.path)
        .map_err(|e| failed(e.to_string()))
}

/// One file given to a tool, checked before a byte of it is copied: one of a content or a path, a
/// name, and no bigger than `cap`. What there is to copy, by its name.
enum Given {
    Data {
        name: String,
        bytes: Vec<u8>,
    },
    Path {
        name: String,
        file: File,
        length: u64,
    },
}

/// `f`, checked (`Given`): a content is decoded once its length is known to be within `cap`, a file
/// is opened (an agent's, from its working folder alone: `open_within`) and its length checked
/// before a byte of it is read. Off the async workers.
fn given(f: FileArg, within: Option<&Path>, cap: Option<u64>) -> Result<Given, ToolError> {
    match (f.data, f.path) {
        (Some(_), Some(_)) => Err(failed(tr!(
            "Donne le contenu du fichier (data) ou son chemin (path), pas les deux.",
            "Give the file’s content (data) or its path (path), not both."
        ))),
        (None, None) => Err(failed(tr!(
            "Donne le contenu du fichier (data) ou son chemin (path).",
            "Give the file’s content (data) or its path (path)."
        ))),
        (Some(data), None) => {
            let Some(raw) = f.name else {
                return Err(failed(tr!(
                    "Un fichier donné par son contenu (data) a besoin d’un nom (name).",
                    "A file given by its content (data) needs a name (name)."
                )));
            };
            let name = name(&raw)?;
            if let Some(cap) = cap.filter(|cap| ticketimages::decoded_len(&data) as u64 > *cap) {
                return Err(too_big(&name, cap));
            }
            // What is wrong in it is not repeated: a content is no text to quote.
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data.trim())
                .map_err(|_| {
                    failed(tr!(
                        "Le contenu de {name} n’est pas du base64 valide.",
                        "The content of {name} isn’t valid base64."
                    ))
                })?;
            Ok(Given::Data { name, bytes })
        }
        (None, Some(path)) => {
            // Its name is the file’s, unless one is given.
            let name = name(f.name.as_deref().unwrap_or(&path))?;
            let (file, length) = open_within(&path, within)?;
            if let Some(cap) = cap.filter(|cap| length > *cap) {
                return Err(too_big(&name, cap));
            }
            Ok(Given::Path { name, file, length })
        }
    }
}

/// The files given to `create_ticket` or added by `update_ticket`, as the drafts the core takes:
/// each copied into the drafts (`drafted` keeps their paths, to remove those the core does not
/// move). `kept`: how many files the ticket keeps. The list is checked whole before a byte is
/// copied: its count (20 with the files kept), each file's form, and each one's length within the
/// size the settings allow. The files named by a path are read off the async workers, from where
/// `caller` may give them (`open_within`); a file that fails says which of the list it is.
pub(super) async fn drafts(
    core: &Arc<Core>,
    caller: &Caller,
    files: Vec<FileArg>,
    kept: usize,
    drafted: &mut Drafted,
) -> Result<Vec<DraftFile>, ToolError> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    // Refused before any is looked at: a long list of paths would read a lot to be refused anyway.
    if kept + files.len() > MAX_FILES {
        return Err(too_many());
    }
    let within = working_folder(core, caller);
    let cap = core.attach_cap();
    let c = core.clone();
    let made = tokio::task::spawn_blocking(move || {
        let checked = files
            .into_iter()
            .enumerate()
            .map(|(i, f)| given(f, within.as_deref(), cap).map_err(|e| in_list(i + 1, e)))
            .collect::<Result<Vec<_>, _>>();
        let checked = match checked {
            Ok(checked) => checked,
            Err(e) => return Err((Vec::new(), e)),
        };
        // Each path is kept as it is made: one that fails leaves the others to be removed.
        let mut out = Vec::with_capacity(checked.len());
        for (i, one) in checked.into_iter().enumerate() {
            let copied = match one {
                Given::Data { name, bytes } => {
                    let length = bytes.len() as u64;
                    to_drafts(&c, &name, bytes.as_slice(), length).map(|p| (name, p))
                }
                Given::Path { name, file, length } => {
                    to_drafts(&c, &name, file, length).map(|p| (name, p))
                }
            };
            match copied {
                Ok(done) => out.push(done),
                Err(e) => return Err((out, in_list(i + 1, e))),
            }
        }
        Ok(out)
    })
    .await
    .map_err(|e| failed(e.to_string()))?;
    let made = match made {
        Ok(made) => made,
        Err((made, e)) => {
            drafted.paths.extend(made.into_iter().map(|(_, p)| p));
            return Err(e);
        }
    };
    Ok(made
        .into_iter()
        .map(|(name, path)| {
            drafted.paths.push(path.clone());
            DraftFile {
                name,
                path,
                ..Default::default()
            }
        })
        .collect())
}

/// The list `Core::ticket_update` takes for `ticket`, once `remove` is taken away from its files
/// and `add` put after them: the files kept (named by their file, as the form keeps them) in their
/// order, then the new ones. The files not named stay: nothing here replaces the whole list. A
/// reference that fits no file fails before anything is copied.
pub(super) async fn amended(
    core: &Arc<Core>,
    caller: &Caller,
    ticket: &Ticket,
    add: Vec<FileArg>,
    remove: &[FileRef],
    drafted: &mut Drafted,
) -> Result<Vec<DraftFile>, ToolError> {
    let gone = select(&ticket.files, remove)?;
    let mut list: Vec<DraftFile> = ticket
        .files
        .iter()
        .enumerate()
        .filter(|(i, _)| !gone.contains(i))
        .map(|(_, f)| DraftFile {
            name: f.name.clone(),
            file: f.file.clone(),
            ..Default::default()
        })
        .collect();
    let kept = list.len();
    list.extend(drafts(core, caller, add, kept, drafted).await?);
    Ok(list)
}

// ---------- the files a split hands down ----------

/// The files of the origin that one ticket of a split takes, as ranks in its list.
pub(super) struct Picks {
    ranks: Vec<usize>,
    /// The ticket named them (`files` given, even empty). Left out, they are all the files of the
    /// origin, and one that cannot be copied is left out too, with a note.
    named: bool,
}

/// The files of `own` that `split_ticket`’s ticket asked for (`asked`, its `files`): all of them
/// when it said nothing, none for an empty list.
pub(super) fn chosen(own: &[TicketFile], asked: Option<&[FileRef]>) -> Result<Picks, ToolError> {
    match asked {
        None => Ok(Picks {
            ranks: (0..own.len()).collect(),
            named: false,
        }),
        Some(refs) => Ok(Picks {
            ranks: select(own, refs)?,
            named: true,
        }),
    }
}

/// The files of a ticket that its split hands down, looked at once before any ticket is made:
/// where each one is and its length, or why it cannot be copied.
#[derive(Default)]
pub(super) struct Handed {
    ready: HashMap<usize, (PathBuf, u64)>,
    why: HashMap<usize, String>,
}

/// Why a file of the origin cannot be copied: it is gone (or unreadable), or it is bigger than the
/// settings now allow (it was joined under a bigger size).
fn why_not(path: Option<PathBuf>, cap: Option<u64>) -> Result<(PathBuf, u64), String> {
    let gone = || {
        tr!(
            "son fichier a disparu ou n’est pas lisible",
            "its file is gone or unreadable"
        )
    };
    let path = path.ok_or_else(gone)?;
    let length = std::fs::metadata(&path).map_err(|_| gone())?.len();
    if let Some(cap) = cap.filter(|cap| length > *cap) {
        let max = size_label(cap);
        return Err(tr!(
            "il dépasse {max}, la taille maximale d’un fichier joint des réglages",
            "it is over {max}, the largest attachment the settings allow"
        ));
    }
    Ok((path, length))
}

impl Handed {
    /// Copies, in the drafts, of the files `pick` takes, for one ticket: new copies, so that the
    /// core moves files of its own into its folder. A file that cannot be copied now fails the call
    /// when the ticket named it; taken by default, it is left out (its rank given back).
    pub(super) async fn copies(
        &self,
        core: &Arc<Core>,
        own: &Ticket,
        pick: &Picks,
        drafted: &mut Drafted,
    ) -> Result<(Vec<DraftFile>, Vec<usize>), ToolError> {
        let wanted: Vec<(usize, PathBuf, u64, String)> = pick
            .ranks
            .iter()
            .filter_map(|i| {
                let (path, length) = self.ready.get(i)?;
                let name = own.files.get(*i)?.name.clone();
                Some((*i, path.clone(), *length, name))
            })
            .collect();
        if wanted.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let c = core.clone();
        let made = tokio::task::spawn_blocking(move || {
            wanted
                .into_iter()
                .map(|(i, path, length, name)| {
                    let copied = File::open(&path)
                        .map_err(|e| failed(e.to_string()))
                        .and_then(|file| to_drafts(&c, &name, file, length));
                    (i, name, copied)
                })
                .collect::<Vec<_>>()
        })
        .await
        .map_err(|e| failed(e.to_string()))?;
        let (mut out, mut missed, mut error) = (Vec::new(), Vec::new(), None);
        for (i, name, copied) in made {
            match copied {
                Ok(path) => {
                    drafted.paths.push(path.clone());
                    out.push(DraftFile {
                        name,
                        path,
                        ..Default::default()
                    });
                }
                Err(e) if pick.named => {
                    error.get_or_insert(e);
                }
                Err(_) => missed.push(i),
            }
        }
        match error {
            Some(e) => Err(e),
            None => Ok((out, missed)),
        }
    }

    /// What is said of the files of `own` that the ticket `key` took by default and that were not
    /// copied (those `missed` at the copy too), one sentence for each.
    pub(super) fn left_out(
        &self,
        own: &Ticket,
        pick: &Picks,
        key: &str,
        missed: &[usize],
    ) -> Vec<String> {
        pick.ranks
            .iter()
            .filter(|i| !self.ready.contains_key(i) || missed.contains(i))
            .filter_map(|i| {
                let f = own.files.get(*i)?;
                let why = self.why.get(i).cloned().unwrap_or_else(|| {
                    tr!(
                        "son fichier a disparu ou n’est pas lisible",
                        "its file is gone or unreadable"
                    )
                });
                let name = shown(&f.name);
                Some(tr!(
                    "{key} : le fichier « {name} » n’a pas été copié, {why}.",
                    "{key}: the file “{name}” wasn’t copied, {why}."
                ))
            })
            .collect()
    }
}

/// Looks, off the async workers, at the files of `own` that the tickets of the split (`picks`, one
/// for each) take: there, and within the size the settings allow. One that is not cannot be copied:
/// a ticket that named it fails the call (its rank in the list, from 0, with the reason); one that
/// took it by default goes without (`Handed::left_out`).
pub(super) async fn hand_down(
    core: &Core,
    own: &Ticket,
    picks: &[Picks],
) -> Result<Handed, (usize, ToolError)> {
    let wanted: BTreeSet<usize> = picks.iter().flat_map(|p| p.ranks.iter().copied()).collect();
    if wanted.is_empty() {
        return Ok(Handed::default());
    }
    let (dir, files, cap) = (
        core.data.ticket_dir(&own.id),
        own.files.clone(),
        core.attach_cap(),
    );
    let handed = tokio::task::spawn_blocking(move || {
        let mut handed = Handed::default();
        for i in wanted {
            let Some(f) = files.get(i) else { continue };
            match why_not(ticketfiles::present(&dir, &f.file), cap) {
                Ok(ready) => {
                    handed.ready.insert(i, ready);
                }
                Err(why) => {
                    handed.why.insert(i, why);
                }
            }
        }
        handed
    })
    .await
    .map_err(|e| (0, failed(e.to_string())))?;
    for (n, pick) in picks.iter().enumerate().filter(|(_, p)| p.named) {
        if let Some(missing) = pick.ranks.iter().find(|i| !handed.ready.contains_key(i)) {
            let name = own
                .files
                .get(*missing)
                .map(|f| shown(&f.name))
                .unwrap_or_default();
            let why = handed.why.get(missing).cloned().unwrap_or_default();
            return Err((
                n,
                failed(tr!(
                    "Le fichier « {name} » du ticket ne peut pas être copié : {why}. Ne le nomme pas dans files (file ou index), ou donne [] pour n’en copier aucun.",
                    "The ticket’s file “{name}” can’t be copied: {why}. Leave it out of files (file or index), or give [] to copy none."
                )),
            ));
        }
    }
    Ok(handed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed(e: Result<impl std::fmt::Debug, ToolError>) -> String {
        match e {
            Err(ToolError::Failed(why)) => why,
            other => panic!("not a failure: {other:?}"),
        }
    }

    #[test]
    fn a_name_is_what_follows_the_last_slash_made_safe_as_the_window_makes_it() {
        assert_eq!(name("rapport.xlsx").unwrap(), "rapport.xlsx");
        assert_eq!(name("../../etc/passwd").unwrap(), "passwd");
        assert_eq!(
            name(r"C:\Users\moi\budget 2026.xlsx").unwrap(),
            "budget 2026.xlsx"
        );
        assert_eq!(name("a\nb.log").unwrap(), "a b.log");
        assert_eq!(name("a\u{202e}b.txt").unwrap(), "ab.txt");
        // What Windows refuses is replaced, and a device name is no name of its own.
        assert_eq!(
            name("a:b?.txt").unwrap(),
            attachfiles::safe_name("a:b?.txt")
        );
        assert_ne!(name("con").unwrap(), "con");
        // A long name is cut, as the window cuts it, never refused.
        let long = format!("{}.tar.gz", "x".repeat(400));
        let cut = name(&long).unwrap();
        assert!(cut.ends_with(".gz") && cut.chars().count() < 400, "{cut}");
        for empty in ["", "   ", "dossier/", r"dossier\", "\u{200b}"] {
            assert!(failed(name(empty)).contains("vide"), "{empty:?}");
        }
    }

    fn had(files: &[&str]) -> Vec<TicketFile> {
        files
            .iter()
            .map(|f| TicketFile {
                name: format!("nom-{f}"),
                file: (*f).into(),
                size: 1,
            })
            .collect()
    }

    fn by_file(file: &str) -> FileRef {
        FileRef {
            file: Some(file.into()),
            index: None,
        }
    }

    fn by_index(index: u32) -> FileRef {
        FileRef {
            file: None,
            index: Some(index),
        }
    }

    #[test]
    fn a_file_is_named_by_its_file_or_its_rank_and_any_other_reference_fails() {
        let files = had(&["11111111/a.zip", "22222222/b.pdf", "33333333/c.log"]);
        assert_eq!(
            select(
                &files,
                &[by_file("33333333/c.log"), by_index(0), by_index(2)]
            )
            .unwrap(),
            [2, 0]
        );
        for hostile in [
            "../state.json",
            "11111111/../../state.json",
            "a.zip",
            "11111111/A.zip",
            "",
            "/etc/passwd",
            r"C:\Windows\win.ini",
        ] {
            let why = failed(select(&files, &[by_file(hostile)]));
            assert!(why.contains("get_ticket"), "{hostile:?}: {why}");
        }
        let why = failed(select(&files, &[by_index(3)]));
        assert!(why.contains("0 à 2"), "{why}");
        assert!(failed(select(&[], &[by_index(0)])).contains("aucun"));
        let both = FileRef {
            file: Some("11111111/a.zip".into()),
            index: Some(0),
        };
        assert!(failed(select(&files, &[both])).contains("pas les deux"));
        let none = FileRef {
            file: None,
            index: None,
        };
        failed(select(&files, &[none]));
        // A hostile reference is cut short, on one line.
        let why = failed(select(
            &files,
            &[by_file(&format!("{}\nfin", "x".repeat(500)))],
        ));
        assert!(why.chars().count() < 300 && !why.contains('\n'), "{why}");
    }

    #[test]
    fn a_part_of_text_is_given_as_text_cut_at_characters() {
        let text = "héhé — fin".as_bytes().to_vec();
        // Whole.
        let all = piece(text.clone(), 0, text.len() as u64);
        assert_eq!(
            (all.encoding, all.content.as_str(), all.offset, all.length),
            ("text", "héhé — fin", 0, text.len())
        );
        // Cut in the middle of « é » at the start (offset 2 is its second byte): skipped.
        let from = piece(text[2..].to_vec(), 2, text.len() as u64);
        assert_eq!((from.encoding, from.offset), ("text", 3));
        assert_eq!(from.content, "hé — fin");
        // Cut in the middle of « — » at the end: left out.
        let dash = "héhé ".len();
        let to = piece(text[..dash + 1].to_vec(), 0, text.len() as u64);
        assert_eq!((to.encoding, to.content.as_str()), ("text", "héhé "));
        assert_eq!(to.length, dash);
    }

    #[test]
    fn a_part_that_is_not_text_is_given_in_base64() {
        let binary = vec![0x50, 0x4b, 0x03, 0x04, 0x00, 0xff, 0xfe];
        let p = piece(binary.clone(), 0, 7);
        assert_eq!(p.encoding, "base64");
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(&p.content)
                .unwrap(),
            binary
        );
        assert_eq!((p.offset, p.length), (0, 7));
        // Valid UTF-8 with a NUL is no text either.
        assert_eq!(piece(b"a\0b".to_vec(), 0, 3).encoding, "base64");
        // An invalid byte that is not where a cut fell: base64, from where it was read.
        let p = piece(vec![b'a', 0xc3, b'b'], 4, 100);
        assert_eq!((p.encoding, p.offset), ("base64", 4));
        // Nothing is text too.
        assert_eq!(piece(Vec::new(), 0, 0).encoding, "text");
    }

    #[test]
    fn a_part_is_read_at_its_offset_or_at_the_end_and_bounded() {
        let dir = crate::paths::test_dir("mcp-files-unit-part");
        let path = dir.join("f.log");
        std::fs::write(&path, b"0123456789").unwrap();
        assert_eq!(
            read_part(&path, None, 4).unwrap(),
            (b"6789".to_vec(), 6, 10)
        );
        assert_eq!(
            read_part(&path, Some(2), 3).unwrap(),
            (b"234".to_vec(), 2, 10)
        );
        assert_eq!(
            read_part(&path, None, 100).unwrap(),
            (b"0123456789".to_vec(), 0, 10)
        );
        // Past the end: nothing, from the end.
        assert_eq!(read_part(&path, Some(50), 3).unwrap(), (Vec::new(), 10, 10));
    }

    #[test]
    fn a_file_of_the_origin_is_handed_down_when_it_is_there_and_within_the_settings() {
        let dir = crate::paths::test_dir("mcp-files-unit-why");
        let path = dir.join("a.bin");
        std::fs::write(&path, vec![0u8; 2 * MB as usize]).unwrap();
        assert_eq!(why_not(Some(path.clone()), None).unwrap().1, 2 * MB);
        assert_eq!(why_not(Some(path.clone()), Some(2 * MB)).unwrap().1, 2 * MB);
        let why = why_not(Some(path.clone()), Some(MB)).unwrap_err();
        assert!(why.contains("1 Mo") && why.contains("réglages"), "{why}");
        assert!(why_not(None, None).unwrap_err().contains("disparu"));
        assert!(why_not(Some(dir.join("absent")), None)
            .unwrap_err()
            .contains("disparu"));
    }
}
