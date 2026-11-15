//! The images of a ticket through the server: the ones the tools attach (`create_ticket`,
//! `update_ticket`), the list `get_ticket` gives, the one `get_ticket_image` reads back and the
//! copies `split_ticket` hands down. Their formats, sizes and count are the window's, checked by
//! the core (`ticketimages::check`) when the ticket is made or saved: nothing here decides them.
//! What the tools add is the ways in (a content in base64, or the path of a file) and the care
//! that a name or a path from the model leads nowhere it should not.

use super::act::{may_act_on, visible_line};
use super::read::own_ticket;
use super::resolve;
use super::tools::{GetTicketImageArgs, ImageArg, ImageRef, ToolError};
use super::Caller;
use crate::core::{Core, MAX_IMAGE, MAX_TOTAL};
use crate::i18n;
use crate::model::{Ticket, TicketImage};
use crate::ticketimages::{self, MAX_IMAGES};
use crate::tickets::DraftImage;
use base64::Engine;
use rmcp::model::ContentBlock;
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// The longest name an image has, in characters.
pub(super) const NAME_MAX: usize = 100;

/// How much of a name or a file the model gave is repeated back to it.
const SHOWN_MAX: usize = 60;

fn failed(why: String) -> ToolError {
    ToolError::Failed(why)
}

/// `e` as the `n`th image of a list given to a tool: a refusal stays one.
fn in_list(n: usize, e: ToolError) -> ToolError {
    let said = |why: String| {
        tr!(
            "Image {n} de la liste : {why}",
            "Image {n} of the list: {why}"
        )
    };
    match e {
        ToolError::Failed(why) => ToolError::Failed(said(why)),
        ToolError::Refused(why) => ToolError::Refused(said(why)),
    }
}

/// The label of an image: what follows the last slash of what was given, on one line and with
/// nothing hidden in it (a direction override could reverse how it reads). The stored file has a
/// name of its own: this one only shows, and goes in the first message of the ticket's agent.
pub(super) fn name(raw: &str) -> Result<String, ToolError> {
    let last = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    let name = visible_line(last);
    let n = name.chars().count();
    if n == 0 {
        return Err(failed(tr!(
            "Le nom de l’image est vide.",
            "The image’s name is empty."
        )));
    }
    if n > NAME_MAX {
        return Err(failed(tr!(
            "Le nom de l’image fait {n} caractères : {NAME_MAX} au plus.",
            "The image’s name is {n} characters long: {NAME_MAX} at most."
        )));
    }
    Ok(name)
}

/// `text` as a refusal repeats it: on one line, cut short.
pub(super) fn shown(text: &str) -> String {
    let line = visible_line(text);
    if line.chars().count() <= SHOWN_MAX {
        return line;
    }
    let mut cut: String = line.chars().take(SHOWN_MAX - 1).collect();
    cut.push('…');
    cut
}

// ---------- the images a ticket has ----------

/// The images of a ticket (`had`) that `refs` name, as ranks in its list, in the order named and
/// each once. A reference is a file or a rank, exactly one of the two, and fits an image of the
/// ticket: any other (a name that leads up, a path, a rank off the list) fails, and nothing is
/// chosen.
pub(super) fn select(had: &[TicketImage], refs: &[ImageRef]) -> Result<Vec<usize>, ToolError> {
    let mut chosen = Vec::with_capacity(refs.len());
    for reference in refs {
        let at = find(had, reference)?;
        if !chosen.contains(&at) {
            chosen.push(at);
        }
    }
    Ok(chosen)
}

/// The rank, in `had`, of the image `reference` names.
fn find(had: &[TicketImage], reference: &ImageRef) -> Result<usize, ToolError> {
    let hint = tr!(
        "Lis get_ticket pour voir les images du ticket (file, index).",
        "Read get_ticket to see the ticket’s images (file, index)."
    );
    match (&reference.file, reference.index) {
        // Compared to the files the ticket has, as they are: a name is never turned into a path.
        (Some(file), None) => had.iter().position(|i| i.file == *file).ok_or_else(|| {
            let file = shown(file);
            failed(tr!(
                "Le ticket n’a pas d’image de fichier « {file} ». {hint}",
                "The ticket has no image of the file “{file}”. {hint}"
            ))
        }),
        (None, Some(index)) => match usize::try_from(index) {
            Ok(at) if at < had.len() => Ok(at),
            _ if had.is_empty() => Err(failed(tr!(
                "Le ticket n’a pas d’image de rang {index} : il n’en a aucune. {hint}",
                "The ticket has no image of rank {index}: it has none. {hint}"
            ))),
            _ => {
                let (count, last) = (had.len(), had.len() - 1);
                Err(failed(tr!(
                    "Le ticket n’a pas d’image de rang {index} : il en a {count} (rangs 0 à {last}). {hint}",
                    "The ticket has no image of rank {index}: it has {count} (ranks 0 to {last}). {hint}"
                )))
            }
        },
        (Some(_), Some(_)) => Err(failed(tr!(
            "Désigne l’image par son nom de fichier (file) ou son rang (index), pas les deux. {hint}",
            "Name the image by its file (file) or its rank (index), not both. {hint}"
        ))),
        (None, None) => Err(failed(tr!(
            "Désigne l’image par son nom de fichier (file) ou son rang (index). {hint}",
            "Name the image by its file (file) or its rank (index). {hint}"
        ))),
    }
}

/// The size of an image’s file; None when it is gone, is no file, or has a name that is not a plain
/// one (`plain_name`): a state edited by hand could name a file anywhere, which is never looked at.
fn size_of(dir: &Path, image: &TicketImage) -> Option<u64> {
    if !ticketimages::plain_name(&image.file) {
        return None;
    }
    std::fs::metadata(dir.join(&image.file))
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len())
}

/// An image of a ticket, as `get_ticket` lists it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ImageView {
    /// Its rank in the ticket’s list, from 0.
    index: usize,
    file: String,
    name: String,
    media_type: String,
    /// In bytes; None when its file is gone.
    size: Option<u64>,
}

/// The images of a ticket whose files are in `dir`, in the ticket’s order: all of them, those whose
/// file is gone too, which keep their rank.
pub(super) fn listed(dir: &Path, images: &[TicketImage]) -> Vec<ImageView> {
    images
        .iter()
        .enumerate()
        .map(|(index, image)| ImageView {
            index,
            file: image.file.clone(),
            name: image.name.clone(),
            media_type: image.media_type.clone(),
            size: size_of(dir, image),
        })
        .collect()
}

/// `get_ticket_image`: the image as an image block, then a text block that says which it is. Not
/// read, and said so, when its file is gone or has no plain name.
pub(super) async fn fetch(
    core: &Core,
    caller: &Caller,
    a: GetTicketImageArgs,
) -> Result<Vec<ContentBlock>, ToolError> {
    let t = match a.ticket.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(asked) => resolve::ticket(core, caller, asked)?,
        None => own_ticket(core, caller)?,
    };
    // The sheet of a ticket (`get_ticket`) is open to all, the files of its images are the files of
    // its project: an agent reads those of the projects it acts on.
    may_act_on(core, caller, &t.project_id)?;
    let index = find(
        &t.images,
        &ImageRef {
            file: a.file,
            index: a.index,
        },
    )?;
    let image = t.images[index].clone();
    let gone = || {
        let name = shown(&image.name);
        failed(tr!(
            "L’image « {name} » ne peut pas être lue : son fichier a disparu ou n’est pas lisible.",
            "The image “{name}” can’t be read: its file is gone or unreadable."
        ))
    };
    let dir = core.data.ticket_dir(&t.id);
    let size = size_of(&dir, &image).ok_or_else(gone)?;
    let wanted = vec![image.clone()];
    let read = tokio::task::spawn_blocking(move || ticketimages::read(&dir, &wanted))
        .await
        .map_err(|e| failed(e.to_string()))?;
    let data = read.into_iter().next().ok_or_else(gone)?;
    let says = json!({
        "ticket": t.key,
        "index": index,
        "file": image.file,
        "name": image.name,
        "mediaType": image.media_type,
        "size": size,
    });
    Ok(vec![
        ContentBlock::image(data.data, data.media_type),
        ContentBlock::text(says.to_string()),
    ])
}

// ---------- the images a tool gives ----------

/// Why an image of a list is not taken.
enum Stop {
    /// This image is not fit: the refusal says which of the list it is.
    Image(ToolError),
    /// With it the list weighs more than a ticket takes. Said as the core says it, for the list.
    Total,
}

impl From<ToolError> for Stop {
    fn from(e: ToolError) -> Self {
        Stop::Image(e)
    }
}

/// `size` more bytes in a list that weighs `total`: refused past what a ticket takes.
fn weigh(total: &mut usize, size: usize) -> Result<(), Stop> {
    *total += size;
    if *total > MAX_TOTAL {
        return Err(Stop::Total);
    }
    Ok(())
}

/// The images given to `create_ticket` or added by `update_ticket`, as the drafts the core takes
/// (its `check` decides the rest: type, size, count, content). An image is a content or a path,
/// not both. The files named by a path are read here, off the async workers, from where `caller`
/// may give them (`open_file`); an image that fails says which of the list it is.
///
/// `kept`: the bytes of the images the ticket keeps. The list is weighed as it goes, a file by its
/// length once it is open and a content by its text, and an image that takes it past what a ticket
/// takes (18 MB) stops it there: that file is not read, and the ones after it are not even looked
/// at (the core would refuse the list anyway, after all of them were read and encoded).
pub(super) async fn drafts(
    core: &Core,
    caller: &Caller,
    images: Vec<ImageArg>,
    kept: usize,
) -> Result<Vec<DraftImage>, ToolError> {
    let within = working_folder(core, caller);
    let mut total = kept;
    let mut out = Vec::with_capacity(images.len());
    for (i, image) in images.into_iter().enumerate() {
        // The core refuses past `MAX_IMAGES` before it looks at the image: a file named there is
        // not read (a long list of paths would read a lot to be refused anyway).
        let read = i < MAX_IMAGES;
        let one = draft(image, within.as_deref(), read, &mut total).await;
        out.push(one.map_err(|stop| match stop {
            Stop::Image(e) => in_list(i + 1, e),
            Stop::Total => failed(ticketimages::too_much(i18n::ui())),
        })?);
    }
    Ok(out)
}

/// The only folder `caller` may give a file from by its path: an agent of Escouade's working folder
/// (`open_within`); None for Claude outside Escouade, which gives any file of the machine.
pub(super) fn working_folder(core: &Core, caller: &Caller) -> Option<PathBuf> {
    match caller {
        Caller::External => None,
        // An agent that is gone has no folder: no file is within it.
        Caller::Agent(id) => Some(
            core.agent(id)
                .map(|h| PathBuf::from(&h.lock().meta.cwd))
                .unwrap_or_default(),
        ),
    }
}

/// One image given to a tool, as a draft; `total` is what the list weighs before it.
async fn draft(
    image: ImageArg,
    within: Option<&Path>,
    read: bool,
    total: &mut usize,
) -> Result<DraftImage, Stop> {
    let media_type = image.media_type.unwrap_or_default();
    match (image.data, image.path) {
        (Some(_), Some(_)) => Err(failed(tr!(
            "Donne le contenu de l’image (data) ou le chemin de son fichier (path), pas les deux.",
            "Give the image’s content (data) or the path of its file (path), not both."
        ))
        .into()),
        (None, None) => Err(failed(tr!(
            "Donne le contenu de l’image (data) ou le chemin de son fichier (path).",
            "Give the image’s content (data) or the path of its file (path)."
        ))
        .into()),
        (Some(data), None) => {
            let Some(raw) = image.name else {
                return Err(failed(tr!(
                    "Une image donnée par son contenu (data) a besoin d’un nom (name).",
                    "An image given by its content (data) needs a name (name)."
                ))
                .into());
            };
            let name = name(&raw)?;
            weigh(total, ticketimages::decoded_len(&data))?;
            Ok(DraftImage {
                name,
                media_type,
                data,
                ..Default::default()
            })
        }
        (None, Some(path)) => {
            // Its name is the file’s, unless one is given.
            let label = name(image.name.as_deref().unwrap_or(&path))?;
            let data = if read {
                let (within, named) = (within.map(Path::to_path_buf), label.clone());
                let (file, length) = tokio::task::spawn_blocking(move || {
                    open_file(&path, within.as_deref(), &named)
                })
                .await
                .map_err(|e| failed(e.to_string()))??;
                // Its length is known before a byte of it is read.
                weigh(total, length as usize)?;
                let named = label.clone();
                let bytes = tokio::task::spawn_blocking(move || read_open(file, &named))
                    .await
                    .map_err(|e| failed(e.to_string()))??;
                base64::engine::general_purpose::STANDARD.encode(bytes)
            } else {
                String::new()
            };
            Ok(DraftImage {
                name: label,
                media_type,
                data,
                ..Default::default()
            })
        }
    }
}

fn unreadable(e: std::io::Error) -> ToolError {
    failed(tr!(
        "Le fichier est introuvable ou illisible : {e}",
        "The file wasn’t found or can’t be read: {e}"
    ))
}

fn not_a_file() -> ToolError {
    failed(tr!(
        "Ce n’est pas un fichier : un dossier ou un périphérique ne se joint pas.",
        "That isn’t a file: a folder or a device can’t be attached."
    ))
}

fn too_big(name: &str) -> ToolError {
    failed(ticketimages::too_big(i18n::ui(), name))
}

/// What an agent is told of a file that is not in its working folder, whether it is there or not:
/// the same words, so that they tell nothing of what is outside.
fn outside() -> ToolError {
    ToolError::Refused(tr!(
        "Un agent d’Escouade ne joint que des fichiers de son dossier de travail : ce fichier n’en fait pas partie, ou il n’existe pas.",
        "An Escouade agent only attaches files from its working folder: this file is not in it, or it does not exist."
    ))
}

/// The image file at `raw`, open, and its length; `name` is its label in a refusal. The path is a
/// full one to a local file: not a relative one, not a network share nor a device (`\\server\share`,
/// `\\?\…`, `//…`: a share would be sent the user’s Windows credentials, a device name can block).
///
/// `within`: the folder an agent of Escouade works in, the only place it may give a file from. That
/// is a safeguard of the server, not a security boundary (an agent that has the run of its machine
/// has other ways to read a file). It is decided on the open file, where its handle really is
/// however a link or a way up led there, and not on the path, which could lead elsewhere between
/// the look and the open. A file outside, one that is not there and a folder outside are answered
/// the same (`outside`); under the folder, what is wrong with a file is said.
///
/// The file must be a regular one of 5 MB at most, said from its length before a byte is read.
pub(super) fn open_file(
    raw: &str,
    within: Option<&Path>,
    name: &str,
) -> Result<(File, u64), ToolError> {
    let (file, length) = open_within(raw, within)?;
    if length > MAX_IMAGE as u64 {
        return Err(too_big(name));
    }
    Ok((file, length))
}

/// `open_file`, whatever the length of the file: the other files of a ticket (`files`) have the
/// limit of the settings, checked by their caller.
pub(super) fn open_within(raw: &str, within: Option<&Path>) -> Result<(File, u64), ToolError> {
    let is_slash = |c: char| c == '/' || c == '\\';
    let mut start = raw.chars();
    if let (Some(a), Some(b)) = (start.next(), start.next()) {
        if is_slash(a) && is_slash(b) {
            return Err(failed(tr!(
                "Un chemin réseau ou de périphérique (qui commence par deux barres) n’est pas accepté : donne le chemin d’un fichier local.",
                "A network or device path (one that starts with two slashes) isn’t accepted: give the path of a local file."
            )));
        }
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(failed(tr!(
            "Le chemin doit être absolu : donne le chemin complet du fichier.",
            "The path must be absolute: give the file’s full path."
        )));
    }
    // A folder that is not there holds no file.
    let home = within
        .map(|folder| folder.canonicalize().map_err(|_| outside()))
        .transpose()?;
    // Whatever stops a file outside the folder (not there, no file, unreadable) is `outside`.
    let hide = |e: ToolError| match &home {
        Some(home) if !place(path).starts_with(home) => outside(),
        _ => e,
    };
    // Looked at before it is opened (a pipe would block the open), and again once it is open.
    let meta = std::fs::metadata(path).map_err(|e| hide(unreadable(e)))?;
    if !meta.is_file() {
        return Err(hide(not_a_file()));
    }
    let file = File::open(path).map_err(|e| hide(unreadable(e)))?;
    if let Some(home) = &home {
        match final_path(&file, path) {
            Ok(at) if at.starts_with(home) => {}
            // Out of it, or not to be told where it is: no file is given.
            _ => return Err(outside()),
        }
    }
    let meta = file.metadata().map_err(unreadable)?;
    if !meta.is_file() {
        return Err(not_a_file());
    }
    Ok((file, meta.len()))
}

/// What the image file `file` (open by `open_file`) holds, 5 MB at most.
pub(super) fn read_open(file: File, name: &str) -> Result<Vec<u8>, ToolError> {
    // A file that grew since is cut one byte past the limit, which is enough to refuse it.
    let mut bytes = Vec::new();
    file.take(MAX_IMAGE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() > MAX_IMAGE {
        return Err(too_big(name));
    }
    Ok(bytes)
}

/// Where `path` is, or would be: the part of it that is there made real (links followed, `..`
/// resolved by the system), the rest after it with its `..` resolved without touching the disk.
fn place(path: &Path) -> PathBuf {
    let parts: Vec<Component> = path.components().collect();
    for cut in (1..=parts.len()).rev() {
        let head: PathBuf = parts[..cut].iter().collect();
        // (A bare drive, `C:`, is no place: it stands for the folder the process is in.)
        if !head.has_root() {
            continue;
        }
        if let Ok(mut real) = head.canonicalize() {
            for part in &parts[cut..] {
                match part {
                    Component::ParentDir => {
                        real.pop();
                    }
                    Component::CurDir => {}
                    other => real.push(other),
                }
            }
            return real;
        }
    }
    path.to_path_buf()
}

/// Where the open file `file` really is, in the form `canonicalize` gives (the folder it is compared to
/// is in that form). `given` is the path it was opened by, for the systems that cannot tell.
#[cfg(windows)]
fn final_path(file: &File, _given: &Path) -> std::io::Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFinalPathNameByHandleW, FILE_NAME_NORMALIZED, VOLUME_NAME_DOS,
    };
    let handle = file.as_raw_handle();
    let mut buf = vec![0u16; 512];
    loop {
        // SAFETY: the handle is open as long as `file` is borrowed, and `buf` has the room it says.
        let n = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                buf.as_mut_ptr(),
                buf.len() as u32,
                FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
            )
        } as usize;
        if n == 0 {
            return Err(std::io::Error::last_os_error());
        }
        // Written: its length, without the end. Too small: the size it needs, with the end.
        if n < buf.len() {
            return Ok(PathBuf::from(std::ffi::OsString::from_wide(&buf[..n])));
        }
        buf.resize(n + 1, 0);
    }
}

#[cfg(target_os = "linux")]
fn final_path(file: &File, _given: &Path) -> std::io::Result<PathBuf> {
    use std::os::fd::AsRawFd;
    std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))
}

#[cfg(target_os = "macos")]
fn final_path(file: &File, _given: &Path) -> std::io::Result<PathBuf> {
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStringExt;
    let mut buf = vec![0u8; libc::PATH_MAX as usize];
    // SAFETY: `F_GETPATH` writes the path of the file, ended by a zero, in a buffer of `PATH_MAX`.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buf.as_mut_ptr()) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    buf.truncate(end);
    Ok(PathBuf::from(std::ffi::OsString::from_vec(buf)))
}

/// The other systems cannot tell it: the path it was opened by is made real instead.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn final_path(_file: &File, given: &Path) -> std::io::Result<PathBuf> {
    given.canonicalize()
}

/// The list `Core::ticket_update` takes for `ticket`, once `remove` is taken away from its images
/// and `add` put after them: the images kept (named by their file, as the form keeps them) in their
/// order, then the new ones. The images not named stay: nothing here replaces the whole list. A
/// reference that fits no image fails before anything is read or written; what the images kept
/// weigh counts in the new ones’ limit.
pub(super) async fn amended(
    core: &Core,
    caller: &Caller,
    ticket: &Ticket,
    add: Vec<ImageArg>,
    remove: &[ImageRef],
) -> Result<Vec<DraftImage>, ToolError> {
    let gone = select(&ticket.images, remove)?;
    let dir = core.data.ticket_dir(&ticket.id);
    let mut weight = 0;
    let mut list = Vec::with_capacity(ticket.images.len());
    for image in ticket
        .images
        .iter()
        .enumerate()
        .filter(|(i, _)| !gone.contains(i))
        .map(|(_, image)| image)
    {
        // As `check` counts them: a file that is not there weighs nothing.
        weight += size_of(&dir, image).unwrap_or(0) as usize;
        list.push(DraftImage {
            file: image.file.clone(),
            ..Default::default()
        });
    }
    list.extend(drafts(core, caller, add, weight).await?);
    Ok(list)
}

// ---------- the images a split hands down ----------

/// The images of the origin that one ticket of a split takes, as ranks in its list.
pub(super) struct Picks {
    ranks: Vec<usize>,
    /// The ticket named them (`images` given, even empty). Left out, they are all the images of the
    /// origin, and one that cannot be copied is left out too, with a note.
    named: bool,
}

/// The images of `own` that `split_ticket`’s ticket asked for (`asked`, its `images`): all of
/// them when it said nothing, none for an empty list.
pub(super) fn chosen(own: &[TicketImage], asked: Option<&[ImageRef]>) -> Result<Picks, ToolError> {
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

/// The images of a ticket that its split hands down, read once, each as a draft (its content in
/// base64, its name and type).
#[derive(Default)]
pub(super) struct Handed(HashMap<usize, DraftImage>);

impl Handed {
    /// The copies for a ticket that takes `picks`: new drafts, so that the core writes files of its
    /// own for it. The images that could not be read are not among them.
    pub(super) fn copies(&self, picks: &Picks) -> Vec<DraftImage> {
        picks
            .ranks
            .iter()
            .filter_map(|i| self.0.get(i).cloned())
            .collect()
    }

    /// What is said of the images of `own` that the ticket `key` took by default and that could not
    /// be copied, one sentence for each.
    pub(super) fn left_out(&self, own: &Ticket, picks: &Picks, key: &str) -> Vec<String> {
        picks
            .ranks
            .iter()
            .filter(|i| !self.0.contains_key(i))
            .filter_map(|i| own.images.get(*i))
            .map(|image| {
                let name = shown(&image.name);
                tr!(
                    "{key} : l’image « {name} » n’a pas été copiée, son fichier a disparu ou n’est pas lisible.",
                    "{key}: the image “{name}” wasn’t copied, its file is gone or unreadable."
                )
            })
            .collect()
    }
}

/// Reads, off the async workers, the images of `own` that the tickets of the split (`picks`, one for
/// each) take. One whose file is gone cannot be copied: a ticket that named it fails the call (its
/// rank in the list, from 0, with the reason); one that took it by default goes without
/// (`Handed::left_out`).
pub(super) async fn hand_down(
    core: &Core,
    own: &Ticket,
    picks: &[Picks],
) -> Result<Handed, (usize, ToolError)> {
    let wanted: BTreeSet<usize> = picks.iter().flat_map(|p| p.ranks.iter().copied()).collect();
    if wanted.is_empty() {
        return Ok(Handed::default());
    }
    let (dir, images) = (core.data.ticket_dir(&own.id), own.images.clone());
    let read = tokio::task::spawn_blocking(move || {
        wanted
            .into_iter()
            .filter_map(|i| {
                let image = images.get(i)?;
                let data = ticketimages::read(&dir, std::slice::from_ref(image))
                    .into_iter()
                    .next()?;
                Some((
                    i,
                    DraftImage {
                        name: data.name,
                        media_type: data.media_type,
                        data: data.data,
                        ..Default::default()
                    },
                ))
            })
            .collect::<HashMap<_, _>>()
    })
    .await
    .map_err(|e| (0, failed(e.to_string())))?;
    for (n, pick) in picks.iter().enumerate().filter(|(_, p)| p.named) {
        if let Some(missing) = pick.ranks.iter().find(|i| !read.contains_key(i)) {
            let name = own
                .images
                .get(*missing)
                .map(|i| shown(&i.name))
                .unwrap_or_default();
            return Err((
                n,
                failed(tr!(
                    "L’image « {name} » du ticket ne peut pas être copiée : son fichier a disparu ou n’est pas lisible. Ne la nomme pas dans images (file ou index), ou donne [] pour n’en copier aucune.",
                    "The ticket’s image “{name}” can’t be copied: its file is gone or unreadable. Leave it out of images (file or index), or give [] to copy none."
                )),
            ));
        }
    }
    Ok(Handed(read))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::MB;
    #[cfg(windows)]
    use crate::mcp::tools_tests::junction;
    use crate::paths::test_dir;
    use serde_json::json;

    /// The content of the file at `raw` as `drafts` takes it: opened, then read.
    fn read_file(raw: &str, within: Option<&Path>, name: &str) -> Result<Vec<u8>, ToolError> {
        let (file, _) = open_file(raw, within, name)?;
        read_open(file, name)
    }

    fn failed(e: Result<impl std::fmt::Debug, ToolError>) -> String {
        match e {
            Err(ToolError::Failed(why)) => why,
            other => panic!("not a failure: {other:?}"),
        }
    }

    fn refused(e: Result<impl std::fmt::Debug, ToolError>) -> String {
        match e {
            Err(ToolError::Refused(why)) => why,
            other => panic!("not a refusal: {other:?}"),
        }
    }

    #[test]
    fn a_name_is_what_follows_the_last_slash_on_one_line_with_nothing_hidden() {
        assert_eq!(name("capture.png").unwrap(), "capture.png");
        assert_eq!(name("  capture.png  ").unwrap(), "capture.png");
        // A hostile name keeps its last part only: the name is a label, the file has one of its own.
        assert_eq!(name("../../etc/passwd.png").unwrap(), "passwd.png");
        assert_eq!(name(r"C:\Users\moi\écran.png").unwrap(), "écran.png");
        assert_eq!(name(r"..\..\x/y\z.png").unwrap(), "z.png");
        assert_eq!(name("a\nb\tc.png").unwrap(), "a b c.png");
        // Nothing that is not drawn: a direction override could reverse how it reads.
        assert_eq!(name("a\u{202e}b\u{200b}c.png").unwrap(), "abc.png");
        // A name that is nothing once cleaned is no name.
        for empty in ["", "   ", "dossier/", r"dossier\", "\u{200b}", "a/\n"] {
            let why = failed(name(empty));
            assert!(why.contains("vide"), "{empty:?}: {why}");
        }
    }

    #[test]
    fn a_name_has_a_hundred_characters_at_most_not_bytes() {
        assert_eq!(NAME_MAX, 100);
        let ok = "é".repeat(100);
        assert_eq!(name(&ok).unwrap(), ok);
        let why = failed(name(&"x".repeat(101)));
        assert!(why.contains("101") && why.contains("100"), "{why}");
        // The limit counts what is kept: a long folder in front is not part of the name.
        let long_folder = format!("{}/ok.png", "d".repeat(500));
        assert_eq!(name(&long_folder).unwrap(), "ok.png");
    }

    fn had(files: &[&str]) -> Vec<TicketImage> {
        files
            .iter()
            .map(|f| TicketImage {
                name: format!("nom-{f}"),
                media_type: "image/png".into(),
                file: (*f).into(),
            })
            .collect()
    }

    fn by_file(file: &str) -> ImageRef {
        ImageRef {
            file: Some(file.into()),
            index: None,
        }
    }

    fn by_index(index: u32) -> ImageRef {
        ImageRef {
            file: None,
            index: Some(index),
        }
    }

    #[test]
    fn an_image_is_named_by_its_file_or_its_rank_from_zero() {
        let images = had(&["aaa.png", "bbb.jpg", "ccc.gif"]);
        assert_eq!(select(&images, &[]).unwrap(), Vec::<usize>::new());
        assert_eq!(
            select(&images, &[by_file("ccc.gif"), by_index(0)]).unwrap(),
            [2, 0]
        );
        // Twice the same one is once, at its first place.
        assert_eq!(
            select(
                &images,
                &[
                    by_index(1),
                    by_file("aaa.png"),
                    by_file("bbb.jpg"),
                    by_index(0)
                ]
            )
            .unwrap(),
            [1, 0]
        );
    }

    #[test]
    fn a_reference_that_fits_no_image_of_the_ticket_is_a_failure_that_sends_to_get_ticket() {
        let images = had(&["aaa.png", "bbb.jpg"]);
        for hostile in [
            "../state.json",
            r"..\state.json",
            "AAA.png",
            "aaa.png ",
            "aaa",
            "",
            "/etc/passwd",
            r"C:\Windows\win.ini",
        ] {
            let why = failed(select(&images, &[by_file(hostile)]));
            assert!(why.contains("get_ticket"), "{hostile:?}: {why}");
        }
        let why = failed(select(&images, &[by_index(2)]));
        assert!(
            why.contains("get_ticket") && why.contains('2') && why.contains("0 à 1"),
            "{why}"
        );
        let why = failed(select(&[], &[by_index(0)]));
        assert!(
            why.contains("get_ticket") && why.contains("aucune"),
            "{why}"
        );
        let both = ImageRef {
            file: Some("aaa.png".into()),
            index: Some(0),
        };
        let why = failed(select(&images, &[both]));
        assert!(why.contains("file") && why.contains("index"), "{why}");
        let neither = ImageRef {
            file: None,
            index: None,
        };
        let why = failed(select(&images, &[neither]));
        assert!(why.contains("file") && why.contains("index"), "{why}");
        // The first bad one fails the lot: nothing is half chosen.
        failed(select(&images, &[by_index(0), by_file("zzz.png")]));
    }

    #[test]
    fn a_hostile_reference_is_cut_short_in_the_failure() {
        let images = had(&["aaa.png"]);
        let why = failed(select(
            &images,
            &[by_file(&format!("{}\nfin", "x".repeat(500)))],
        ));
        assert!(why.chars().count() < 300 && !why.contains('\n'), "{why}");
    }

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-capture";

    #[test]
    fn a_file_is_read_by_its_absolute_path() {
        let dir = test_dir("mcp-images-unit-read");
        let file = dir.join("capture.png");
        std::fs::write(&file, PNG).unwrap();
        let path = file.to_string_lossy().to_string();
        assert_eq!(read_file(&path, None, "capture.png").unwrap(), PNG);
        // The very limit is taken.
        let big = dir.join("big.png");
        std::fs::write(&big, vec![7; 5 * MB]).unwrap();
        assert_eq!(
            read_file(&big.to_string_lossy(), None, "big.png")
                .unwrap()
                .len(),
            5 * MB
        );
    }

    #[test]
    fn a_path_that_is_not_absolute_or_leads_off_the_machine_is_refused_before_the_disk_is_touched()
    {
        let dir = test_dir("mcp-images-unit-paths");
        std::fs::write(dir.join("capture.png"), PNG).unwrap();
        for relative in [
            "capture.png",
            "./capture.png",
            "../capture.png",
            "",
            "~/a.png",
        ] {
            let why = failed(read_file(relative, None, "n.png"));
            assert!(why.contains("absolu"), "{relative:?}: {why}");
        }
        // The folder of the process is no place to look in either, whatever the system makes of it.
        #[cfg(windows)]
        for relative in [r"C:capture.png", r"\capture.png"] {
            let why = failed(read_file(relative, None, "n.png"));
            assert!(why.contains("absolu"), "{relative:?}: {why}");
        }
        // A network share would be sent the user's credentials; a device name blocks.
        for network in [
            r"\\serveur\partage\a.png",
            "//x/y.png",
            r"\\?\C:\a.png",
            r"\\.\pipe\a",
            r"/\x\y.png",
            r"\/x/y.png",
        ] {
            let why = failed(read_file(network, None, "n.png"));
            assert!(why.contains("réseau"), "{network:?}: {why}");
        }
    }

    #[test]
    fn a_file_that_is_not_there_or_is_not_a_file_or_is_too_big_is_a_failure() {
        let dir = test_dir("mcp-images-unit-bad");
        let gone = dir.join("gone.png");
        let why = failed(read_file(&gone.to_string_lossy(), None, "gone.png"));
        assert!(why.contains("introuvable"), "{why}");
        let why = failed(read_file(&dir.to_string_lossy(), None, "dossier"));
        assert!(why.contains("pas un fichier"), "{why}");
        // Over the limit by a byte: refused as the window refuses it, by name.
        let big = dir.join("big.png");
        std::fs::write(&big, vec![7; 5 * MB + 1]).unwrap();
        let why = failed(read_file(&big.to_string_lossy(), None, "photo.png"));
        assert_eq!(why, "photo.png dépasse 5 Mo");
        // Whatever the length says, a file is never read past the limit.
        let huge = dir.join("huge.png");
        let f = std::fs::File::create(&huge).unwrap();
        f.set_len(64 * MB as u64).unwrap();
        let why = failed(read_file(&huge.to_string_lossy(), None, "huge.png"));
        assert_eq!(why, "huge.png dépasse 5 Mo");
    }

    #[test]
    fn an_agent_only_reads_the_files_of_its_working_folder() {
        let dir = test_dir("mcp-images-unit-within");
        let work = dir.join("work");
        std::fs::create_dir_all(work.join("sub")).unwrap();
        std::fs::write(work.join("sub").join("in.png"), PNG).unwrap();
        std::fs::write(dir.join("out.png"), PNG).unwrap();
        let ok = work.join("sub").join("in.png");
        assert_eq!(
            read_file(&ok.to_string_lossy(), Some(&work), "in.png").unwrap(),
            PNG
        );
        // Outside, and a way up that comes back out: refused (not failed), with the reason.
        let out = dir.join("out.png");
        let why = refused(read_file(&out.to_string_lossy(), Some(&work), "out.png"));
        assert!(why.contains("dossier de travail"), "{why}");
        let up = work.join("sub").join("..").join("..").join("out.png");
        refused(read_file(&up.to_string_lossy(), Some(&work), "out.png"));
        // A sibling whose name starts like the folder's is not inside it.
        let twin = dir.join("work-twin");
        std::fs::create_dir_all(&twin).unwrap();
        std::fs::write(twin.join("t.png"), PNG).unwrap();
        refused(read_file(
            &twin.join("t.png").to_string_lossy(),
            Some(&work),
            "t.png",
        ));
        // A folder that is not there is a place no file is in.
        refused(read_file(
            &ok.to_string_lossy(),
            Some(&dir.join("nowhere")),
            "in.png",
        ));
        // Not there at all, inside: a failure, as for anyone.
        let gone = work.join("gone.png");
        failed(read_file(&gone.to_string_lossy(), Some(&work), "gone.png"));
    }

    const OUTSIDE: &str = "Un agent d’Escouade ne joint que des fichiers de son dossier de travail : ce fichier n’en fait pas partie, ou il n’existe pas.";

    #[test]
    fn an_agent_is_told_the_same_of_a_file_outside_its_folder_whether_it_exists_or_not() {
        let dir = test_dir("mcp-images-unit-same-words");
        let work = dir.join("work");
        std::fs::create_dir_all(work.join("sub")).unwrap();
        std::fs::create_dir_all(dir.join("outdir")).unwrap();
        std::fs::write(dir.join("out.png"), PNG).unwrap();
        std::fs::write(dir.join("big.png"), vec![7; 5 * MB + 1]).unwrap();
        let said =
            |path: PathBuf| refused(read_file(&path.to_string_lossy(), Some(&work), "x.png"));
        assert_eq!(said(dir.join("out.png")), OUTSIDE);
        // Not there, a folder, over the limit, a way up that comes back out, a place that is not
        // there either: the same words, which tell nothing of what is outside.
        for other in [
            dir.join("gone.png"),
            dir.join("outdir"),
            dir.join("big.png"),
            dir.join("nowhere").join("gone.png"),
            work.join("sub").join("..").join("..").join("gone.png"),
            work.join("..").join("gone.png"),
            work.join("..").join("outdir"),
        ] {
            assert_eq!(said(other.clone()), OUTSIDE, "{other:?}");
        }
        // Inside, nothing is hidden: not there, a folder, over the limit, or a way up that stays in.
        std::fs::write(work.join("big.png"), vec![7; 5 * MB + 1]).unwrap();
        let inside =
            |path: PathBuf| failed(read_file(&path.to_string_lossy(), Some(&work), "x.png"));
        assert!(inside(work.join("sub").join("gone.png")).contains("introuvable"));
        assert!(inside(work.join("sub").join("..").join("gone.png")).contains("introuvable"));
        assert!(inside(work.join("nowhere").join("gone.png")).contains("introuvable"));
        assert!(inside(work.join("sub")).contains("pas un fichier"));
        assert_eq!(inside(work.join("big.png")), "x.png dépasse 5 Mo");
    }

    #[test]
    fn the_final_path_of_an_open_file_is_where_it_really_is() {
        let dir = test_dir("mcp-images-unit-final");
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::write(dir.join("a").join("f.png"), PNG).unwrap();
        // Opened by a path with a way up in it: the handle knows its own.
        let by_a_way = dir.join("a").join("..").join("a").join("f.png");
        let file = std::fs::File::open(&by_a_way).unwrap();
        assert_eq!(
            final_path(&file, &by_a_way).unwrap(),
            dir.join("a").join("f.png").canonicalize().unwrap()
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_out_of_the_working_folder_does_not_lead_an_agent_out() {
        let dir = test_dir("mcp-images-unit-junction");
        let (work, outside, inner) = (
            dir.join("work"),
            dir.join("outside"),
            dir.join("work").join("inner"),
        );
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(outside.join("secret.png"), PNG).unwrap();
        std::fs::write(inner.join("in.png"), PNG).unwrap();
        // Both are in the working folder, one leads out of it and the other stays in.
        let _out = junction(&work.join("lien"), &outside);
        let _in = junction(&work.join("lien-interne"), &inner);
        let through = work.join("lien").join("secret.png");
        assert_eq!(
            refused(read_file(
                &through.to_string_lossy(),
                Some(&work),
                "secret.png"
            )),
            OUTSIDE
        );
        // Where the open handle really is: the target’s.
        let file = std::fs::File::open(&through).unwrap();
        assert_eq!(
            final_path(&file, &through).unwrap(),
            outside.join("secret.png").canonicalize().unwrap()
        );
        // Anyone else follows it, as it is a file of the machine; one that stays in is no obstacle.
        assert_eq!(
            read_file(&through.to_string_lossy(), None, "secret.png").unwrap(),
            PNG
        );
        let kept = work.join("lien-interne").join("in.png");
        assert_eq!(
            read_file(&kept.to_string_lossy(), Some(&work), "in.png").unwrap(),
            PNG
        );
        // Not there beyond the link that leads out: nothing is said of what is out there.
        let gone = work.join("lien").join("gone.png");
        assert_eq!(
            refused(read_file(&gone.to_string_lossy(), Some(&work), "gone.png")),
            OUTSIDE
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_working_folder_does_not_lead_an_agent_out() {
        let dir = test_dir("mcp-images-unit-link");
        let work = dir.join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(dir.join("secret.png"), PNG).unwrap();
        let link = work.join("link.png");
        std::os::unix::fs::symlink(dir.join("secret.png"), &link).unwrap();
        refused(read_file(&link.to_string_lossy(), Some(&work), "link.png"));
        // Anyone else follows it: it is a file of the machine.
        assert_eq!(
            read_file(&link.to_string_lossy(), None, "link.png").unwrap(),
            PNG
        );
    }

    #[test]
    fn a_ticket_lists_its_images_with_the_size_of_each_file() {
        let dir = test_dir("mcp-images-unit-listed");
        std::fs::write(dir.join("aaa.png"), PNG).unwrap();
        std::fs::write(dir.join("ccc.gif"), b"GIF89a").unwrap();
        // A folder with the name of a file is none; a name that is not a plain one is never looked at,
        // though there is a file where it leads.
        std::fs::create_dir_all(dir.join("ddd.png")).unwrap();
        std::fs::write(dir.parent().unwrap().join("x.png"), PNG).unwrap();
        let images: Vec<TicketImage> = [
            ("a.png", "aaa.png"),
            ("b.png", "bbb.png"),
            ("c.gif", "ccc.gif"),
            ("d.png", "ddd.png"),
            ("x.png", "../x.png"),
        ]
        .iter()
        .map(|(name, file)| TicketImage {
            name: (*name).into(),
            media_type: if name.ends_with("gif") {
                "image/gif".into()
            } else {
                "image/png".into()
            },
            file: (*file).into(),
        })
        .collect();
        let sizes: Vec<_> = listed(&dir, &images)
            .into_iter()
            .map(|v| serde_json::to_value(v).unwrap())
            .collect();
        assert_eq!(
            sizes,
            [
                json!({ "index": 0, "file": "aaa.png", "name": "a.png", "mediaType": "image/png", "size": PNG.len() }),
                json!({ "index": 1, "file": "bbb.png", "name": "b.png", "mediaType": "image/png", "size": null }),
                json!({ "index": 2, "file": "ccc.gif", "name": "c.gif", "mediaType": "image/gif", "size": 6 }),
                json!({ "index": 3, "file": "ddd.png", "name": "d.png", "mediaType": "image/png", "size": null }),
                json!({ "index": 4, "file": "../x.png", "name": "x.png", "mediaType": "image/png", "size": null }),
            ]
        );
        assert!(listed(&dir, &[]).is_empty());
    }
}
