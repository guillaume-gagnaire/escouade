//! The files other than images joined in a ticket's form (ESC-95): copies the form made at once in
//! the drafts (`attachments/_drafts/<uniq>/<name>`), moved into the ticket's folder when it is
//! saved (`<ticket folder>/files/<uniq>/<name>`, `DataDir::ticket_files`). The ticket's agent is
//! given their paths with its first message, and its Claude Code reads the folder without a
//! question (`--add-dir`). The images the chat reads stay the ticket's images (`ticketimages`).

use crate::attachfiles::{self, AttachedFile};
use crate::core::Core;
use crate::engine::Program;
use crate::i18n::{self, Lang};
use crate::model::{Ticket, TicketFile};
use crate::paths::TICKET_FILES;
use crate::tickets::DraftFile;
use anyhow::{anyhow, bail, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The most files a ticket takes (as many as images): its first message has a line for each, and
/// « Reprendre » sends that message again.
pub(crate) const MAX_FILES: usize = 20;
const MB: u64 = 1 << 20;

/// A file of a form, checked.
#[derive(Debug)]
pub(crate) enum Checked {
    /// One the ticket has.
    Kept(TicketFile),
    /// A copy of the drafts, to move into the ticket's folder: where it really is (links
    /// resolved), its name made safe and its size.
    New {
        source: PathBuf,
        name: String,
        size: u64,
    },
}

/// Where a ticket's file is kept: `<uniq>/<name>`, the folder of the file and its safe name.
/// The file of a ticket comes from the saved state, which may be edited by hand: any other form
/// (a way up, a path, another drive, a name Windows would read as a device) is never read nor
/// removed, whatever it points to.
pub(crate) fn valid_file(file: &str) -> Option<(&str, &str)> {
    let (uniq, name) = file.split_once('/')?;
    let folder = !uniq.is_empty()
        && uniq
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    // A name that `safe_name` leaves as it is has no separator, no way up, no device name.
    (folder && !name.is_empty() && attachfiles::safe_name(name) == name).then_some((uniq, name))
}

/// The ticket's file `file` as a path under the ticket's folder `dir`; None unless it is a valid
/// one. Made of its names, so that the separators are the system's.
pub(crate) fn path_of(dir: &Path, file: &str) -> Option<PathBuf> {
    let (uniq, name) = valid_file(file)?;
    Some(dir.join(TICKET_FILES).join(uniq).join(name))
}

/// Where the ticket's file `file` (in the ticket's folder `dir`) really is, when it is there: a
/// file of the folder, never a link that leaves it nor a name that is not a valid one. None when
/// it is gone (the purge of `attachKeepDays`).
pub(crate) fn present(dir: &Path, file: &str) -> Option<PathBuf> {
    attachfiles::copy_in(&dir.join(TICKET_FILES), &path_of(dir, file)?)
}

/// The refusal of a list of more than `MAX_FILES` files, for the core and for whatever refuses
/// one before it copies the files.
pub(crate) fn too_many(lang: Lang) -> String {
    tr_in!(
        lang,
        "Un ticket prend {max} fichiers au plus.",
        "A ticket takes {max} files at most.",
        max = MAX_FILES
    )
}

fn outside(lang: Lang, file: &str) -> anyhow::Error {
    anyhow!(tr_in!(
        lang,
        "chemin hors du dossier : {file}",
        "path outside the folder: {file}"
    ))
}

pub(crate) fn too_big(lang: Lang, name: &str, cap: u64) -> anyhow::Error {
    anyhow!(tr_in!(
        lang,
        "{name} dépasse {max}",
        "{name} is over {max}",
        max = tr_in!(lang, "{n} Mo", "{n} MB", n = cap / MB)
    ))
}

/// The list of a ticket's form, checked against what the ticket has (`had`, its files in `dir`).
/// A file kept is one of the ticket's own and nothing else: it is left out when its copy is gone
/// (the purge of `attachKeepDays`), and refused when its name would lead out of the ticket's
/// folder. A new one is a copy of the drafts (`drafts`), found where it really is: never a path
/// elsewhere, nor a link that leaves them; no bigger than `cap`, when there is one. There are 20
/// at most.
pub(crate) fn check(
    drafts: &Path,
    dir: &Path,
    had: &[TicketFile],
    files: &[DraftFile],
    cap: Option<u64>,
) -> Result<Vec<Checked>> {
    check_in(i18n::ui(), drafts, dir, had, files, cap)
}

fn check_in(
    lang: Lang,
    drafts: &Path,
    dir: &Path,
    had: &[TicketFile],
    files: &[DraftFile],
    cap: Option<u64>,
) -> Result<Vec<Checked>> {
    let folder = dir.join(TICKET_FILES);
    let mut out: Vec<Checked> = Vec::new();
    for (n, f) in files.iter().enumerate() {
        // Counted before anything of this one is looked at.
        if n >= MAX_FILES {
            bail!(too_many(lang));
        }
        if !f.file.is_empty() {
            let Some(own) = had.iter().find(|h| h.file == f.file) else {
                bail!(tr_in!(
                    lang,
                    "« {name} » n’est pas un fichier de ce ticket.",
                    "“{name}” isn’t a file of this ticket.",
                    name = f.name
                ));
            };
            let Some(path) = path_of(dir, &own.file) else {
                return Err(outside(lang, &own.file));
            };
            // Named twice: kept once.
            if out
                .iter()
                .any(|c| matches!(c, Checked::Kept(k) if k.file == own.file))
            {
                continue;
            }
            // Its copy is there: a file of the folder. Gone (the purge), or a link that leaves
            // the folder, it is left out of the list, and never read.
            if attachfiles::copy_in(&folder, &path).is_none() {
                log::warn!("{}: file of a ticket not kept: not there", path.display());
                continue;
            }
            out.push(Checked::Kept(own.clone()));
            continue;
        }
        let path = Path::new(&f.path);
        let Some(source) = attachfiles::copy_in(drafts, path) else {
            // Gone: the drafts are emptied after a day.
            let gone = std::fs::symlink_metadata(path)
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
            if gone {
                bail!(tr_in!(
                    lang,
                    "« {name} » n’est plus là : joins-le de nouveau.",
                    "“{name}” is gone: join it again.",
                    name = f.name
                ));
            }
            bail!(tr_in!(
                lang,
                "« {name} » n’est pas un fichier joint dans ce formulaire.",
                "“{name}” isn’t a file joined in this form.",
                name = f.name
            ));
        };
        // Named twice: moved once.
        if out
            .iter()
            .any(|c| matches!(c, Checked::New { source: s, .. } if *s == source))
        {
            continue;
        }
        let name = attachfiles::safe_name(
            &source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
        let size = std::fs::metadata(&source)
            .map_err(|e| {
                anyhow!(tr_in!(
                    lang,
                    "{name} n’a pas pu être lu : {e}",
                    "{name} couldn’t be read: {e}"
                ))
            })?
            .len();
        if let Some(cap) = cap.filter(|cap| size > *cap) {
            return Err(too_big(lang, &name, cap));
        }
        out.push(Checked::New { source, name, size });
    }
    Ok(out)
}

/// Moves the new files of `checked` into `dir`, each in a folder of its own (`<uniq>/<name>`: two
/// files of one name do not meet); gives the ticket's list, in the form's order. What it moved
/// goes again when one fails: nothing of the ticket is changed.
pub(crate) fn place(dir: &Path, checked: Vec<Checked>) -> Result<Vec<TicketFile>> {
    place_with(dir, checked, |from, to| {
        move_with(from, to, |a, b| std::fs::rename(a, b))
    })
}

/// `place`, the file moved by `mv` (a test makes it fail midway).
fn place_with(
    dir: &Path,
    checked: Vec<Checked>,
    mv: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<Vec<TicketFile>> {
    let root = dir.join(TICKET_FILES);
    let mut placed = Vec::new();
    let out = checked
        .into_iter()
        .map(|c| match c {
            Checked::Kept(file) => Ok(file),
            Checked::New { source, name, size } => {
                let uniq = attachfiles::uniq();
                let file = format!("{uniq}/{name}");
                // Noted before it is moved: one that fails midway leaves half a file, which
                // `let_go` must take (it tolerates a file that was never there).
                placed.push(file.clone());
                std::fs::create_dir_all(root.join(&uniq))
                    .and_then(|()| mv(&source, &root.join(&uniq).join(&name)))
                    .map_err(|e| {
                        anyhow!(tr!(
                            "{name} n’a pas pu être enregistré : {e}",
                            "{name} couldn’t be saved: {e}"
                        ))
                    })?;
                // The copy's own folder in the drafts, empty now.
                if let Some(folder) = source.parent() {
                    let _ = std::fs::remove_dir(folder);
                }
                Ok(TicketFile { name, file, size })
            }
        })
        .collect::<Result<Vec<_>>>();
    if out.is_err() {
        let_go(dir, &placed);
    }
    out
}

/// `from` moved to `to`: renamed (`rename`), or, when the system refuses (another volume), copied
/// and then removed.
fn move_with(
    from: &Path,
    to: &Path,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    if rename(from, to).is_ok() {
        return Ok(());
    }
    if let Err(e) = std::fs::copy(from, to) {
        // No half file is left.
        let _ = std::fs::remove_file(to);
        return Err(e);
    }
    // The copy is there: a draft that stays is emptied after a day (`maintain`).
    if let Err(e) = std::fs::remove_file(from) {
        log::warn!("{}: not removed after its copy: {e}", from.display());
    }
    Ok(())
}

/// Removes these files of `dir`'s `files` folder, with their folders once empty, and the ticket's
/// folder once it has nothing left. Only them: another save of the ticket may have written its own
/// meanwhile. A name that is not a valid one is left alone, and so is a file that is no file of
/// the folder (a link that leaves it).
pub(crate) fn let_go(dir: &Path, files: &[String]) {
    let root = dir.join(TICKET_FILES);
    for file in files {
        let Some(path) = path_of(dir, file) else {
            log::warn!("{file}: not a valid file name, not removed");
            continue;
        };
        if let Some(real) = attachfiles::copy_in(&root, &path) {
            if let Err(e) = std::fs::remove_file(&real) {
                log::warn!("{}: not removed: {e}", real.display());
            }
        }
        // Refused while it has files.
        if let Some(folder) = path.parent() {
            let _ = std::fs::remove_dir(folder);
        }
    }
    let _ = std::fs::remove_dir(&root);
    let _ = std::fs::remove_dir(dir);
}

/// What a ticket's first message says of its files (`files`, in `dir`), in `lang`: a line for each
/// that is there, « Fichier joint : <name>, <path> » (the path its `program` reads it by), and
/// after them, one sentence for those that are not (the purge took them): said as an image that is
/// missing is. Empty without a file.
pub(crate) fn message(
    lang: Lang,
    program: Option<&Program>,
    dir: &Path,
    files: &[TicketFile],
) -> String {
    let root = dir.join(TICKET_FILES);
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for f in files {
        // There: a file of the folder, a link that leaves it is none.
        match path_of(dir, &f.file).filter(|p| attachfiles::copy_in(&root, p).is_some()) {
            Some(path) => present.push((f, path)),
            None => missing.push(f.name.clone()),
        }
    }
    said(lang, program, &present, &missing)
}

/// `message`, for the files `present` (with where they are) and the names of those `missing`.
fn said(
    lang: Lang,
    program: Option<&Program>,
    present: &[(&TicketFile, PathBuf)],
    missing: &[String],
) -> String {
    let mut parts = Vec::new();
    if !present.is_empty() {
        let files: Vec<AttachedFile> = present
            .iter()
            .map(|(f, path)| AttachedFile {
                name: f.name.clone(),
                path: path.to_string_lossy().into_owned(),
                claude_path: attachfiles::claude_path_of(program, path),
                size: f.size,
                origin: false,
            })
            .collect();
        parts.push(attachfiles::message_lines(lang, &files));
    }
    if !missing.is_empty() {
        let names = missing
            .iter()
            .map(|n| attachfiles::one_line(n))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(tr_n_in!(
            lang,
            missing.len(),
            "Fichier absent du dossier du ticket, non joint : {names}.",
            "Fichiers absents du dossier du ticket, non joints : {names}.",
            "File missing from the ticket’s folder, not attached: {names}.",
            "Files missing from the ticket’s folder, not attached: {names}."
        ));
    }
    parts.join("\n\n")
}

/// The purge of `attachKeepDays` for the tickets' files (`tickets` holds the folder of each): the
/// files of `<ticket>/files/<uniq>/<name>` older than `keep_days` days go, with their folders once
/// empty. The ticket keeps its entry for each: its agent is told the file is missing. 0: they stay
/// as long as their ticket does. The ticket's images are never touched.
pub(crate) fn maintain(tickets: &Path, keep_days: u32) {
    if keep_days == 0 {
        return;
    }
    let limit = Duration::from_secs(u64::from(keep_days) * 24 * 3600);
    let Ok(entries) = std::fs::read_dir(tickets) else {
        return;
    };
    for ticket in entries.flatten() {
        if !ticket.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let root = ticket.path().join(TICKET_FILES);
        // No copy of the tickets is ever under way: none is written in place.
        attachfiles::tidy_owner(&root, Some(limit), &HashSet::new());
        // Nothing left of a ticket that had only files; refused while it has any.
        let _ = std::fs::remove_dir(&root);
        let _ = std::fs::remove_dir(ticket.path());
    }
}

impl Core {
    /// The folder of the files of the ticket of an agent, which it reads without a question
    /// (`--add-dir` for Claude Code); made here first. None for an agent that has no
    /// ticket, a ticket that has no file (its folder is not made for nothing), or when the folder
    /// cannot be made (the agent starts, with its files asked for).
    pub(crate) fn ticket_read_dir(&self, agent_id: &str) -> Option<std::path::PathBuf> {
        let ticket_id = self.agent(agent_id).ok()?.lock().meta.ticket_id.clone()?;
        // Read once the agent's lock is let go: the tickets come before the agents.
        if self.ticket(&ticket_id).ok()?.files.is_empty() {
            return None;
        }
        let dir = self.data.ticket_files(&ticket_id);
        match std::fs::create_dir_all(&dir) {
            Ok(()) => Some(dir),
            Err(e) => {
                log::warn!("agent {agent_id}: {} not made: {e}", dir.display());
                None
            }
        }
    }

    /// `text`, the first message of the agent `agent_id` of the ticket `t`, followed by what it
    /// says of the ticket's files (`message`): the path each is read by, its program's own.
    pub(crate) fn with_ticket_files(
        &self,
        lang: Lang,
        agent_id: &str,
        t: &Ticket,
        text: String,
    ) -> String {
        if t.files.is_empty() {
            return text;
        }
        let program = self.program_of(agent_id);
        let note = message(
            lang,
            program.as_ref(),
            &self.data.ticket_dir(&t.id),
            &t.files,
        );
        format!("{}\n\n{note}", text.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang::{En, Fr};
    use crate::paths::{make_dir_link, test_dir};
    use std::time::SystemTime;

    /// A copy the form made in the drafts: `<drafts>/<uniq>/<name>`.
    fn draft_copy(drafts: &Path, uniq: &str, name: &str, bytes: &[u8]) -> PathBuf {
        let folder = drafts.join(uniq);
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn new_file(name: &str, path: &Path) -> DraftFile {
        DraftFile {
            name: name.into(),
            path: path.to_string_lossy().into_owned(),
            ..Default::default()
        }
    }

    fn keep(file: &str) -> DraftFile {
        DraftFile {
            name: "form’s name".into(),
            file: file.into(),
            ..Default::default()
        }
    }

    /// A file the ticket has: `<dir>/files/<file>`, written.
    fn own(dir: &Path, name: &str, file: &str, bytes: &[u8]) -> TicketFile {
        let path = path_of(dir, file).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        TicketFile {
            name: name.into(),
            file: file.into(),
            size: bytes.len() as u64,
        }
    }

    fn refusal(r: Result<Vec<Checked>>) -> String {
        match r {
            Err(e) => e.to_string(),
            Ok(c) => panic!("accepted: {c:?}"),
        }
    }

    /// The files under `dir`, as `<uniq>/<name>`, sorted.
    fn listed(dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        for uniq in std::fs::read_dir(dir.join(TICKET_FILES))
            .into_iter()
            .flatten()
            .flatten()
        {
            for file in std::fs::read_dir(uniq.path()).unwrap().flatten() {
                out.push(format!(
                    "{}/{}",
                    uniq.file_name().to_string_lossy(),
                    file.file_name().to_string_lossy()
                ));
            }
        }
        out.sort();
        out
    }

    #[test]
    fn only_a_folder_name_and_a_safe_file_name_make_a_file_of_the_tickets_folder() {
        for (file, split) in [
            ("4f2a9c01/rapport.xlsx", ("4f2a9c01", "rapport.xlsx")),
            ("ab-12_cd/a b (1).tar.gz", ("ab-12_cd", "a b (1).tar.gz")),
            ("4f2a9c01/.gitignore", ("4f2a9c01", ".gitignore")),
        ] {
            assert_eq!(valid_file(file), Some(split), "{file}");
        }
        for other in [
            "",
            "rapport.xlsx",
            "/",
            "4f2a9c01/",
            "/rapport.xlsx",
            "../rapport.xlsx",
            "../../outside.txt",
            "4f2a9c01/../rapport.xlsx",
            "4f2a9c01/..",
            "4f2a9c01/.",
            "./rapport.xlsx",
            "..\\rapport.xlsx",
            "4f2a9c01\\rapport.xlsx",
            "4f2a9c01/a/b.txt",
            "C:/Windows/win.ini",
            "C:\\Windows\\win.ini",
            "/etc/passwd",
            "4f2a9c01/con",
            "4f2a9c01/a:b",
            "4f2a9c01/a.",
            "4f2a9c01/a\nb",
            "4f 2a/rapport.xlsx",
            "4f2a9c01/\u{202e}fdp.exe",
        ] {
            assert_eq!(valid_file(other), None, "{other:?}");
        }
        // Where it is: the path of the ticket's file is made of the names, with no separator of
        // another kind in it.
        let dir = Path::new("tickets").join("t1");
        assert_eq!(
            path_of(&dir, "4f2a9c01/rapport.xlsx"),
            Some(dir.join("files").join("4f2a9c01").join("rapport.xlsx"))
        );
        assert_eq!(path_of(&dir, "../x"), None);
    }

    #[test]
    fn a_file_of_the_drafts_is_a_new_file_of_its_real_name_and_size() {
        let root = test_dir("tf-new");
        let (drafts, dir) = (root.join("drafts"), root.join("t1"));
        let path = draft_copy(&drafts, "11111111", "budget 2026.xlsx", b"xlsx!");
        let checked = check_in(
            Fr,
            &drafts,
            &dir,
            &[],
            &[new_file("un autre nom", &path)],
            None,
        )
        .unwrap();
        assert_eq!(checked.len(), 1);
        // Its own name, as its copy was made (never the form's), and what it weighs now.
        match &checked[0] {
            Checked::New { source, name, size } => {
                assert_eq!(
                    std::fs::canonicalize(source).unwrap(),
                    std::fs::canonicalize(&path).unwrap()
                );
                assert_eq!((name.as_str(), *size), ("budget 2026.xlsx", 5));
            }
            other => panic!("{other:?}"),
        }
        // Nothing is moved by a check.
        assert!(path.exists());
        // An empty file is a file.
        let empty = draft_copy(&drafts, "22222222", "vide.txt", b"");
        assert_eq!(
            check_in(
                Fr,
                &drafts,
                &dir,
                &[],
                &[new_file("vide.txt", &empty)],
                None
            )
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn a_path_that_is_not_a_copy_of_the_drafts_is_refused() {
        let root = test_dir("tf-outside");
        let (drafts, dir) = (root.join("attachments").join("_drafts"), root.join("t1"));
        let mine = draft_copy(&drafts, "11111111", "a.zip", b"zip");
        // What belongs to an agent, or to the user.
        let agent = draft_copy(
            &root.join("attachments").join("a1"),
            "22222222",
            "b.zip",
            b"zip",
        );
        let elsewhere = root.join("own.xlsx");
        std::fs::write(&elsewhere, b"mine").unwrap();
        // A way up out of the drafts, back to a file that exists.
        let up = drafts
            .join("11111111")
            .join("..")
            .join("..")
            .join("a1")
            .join("22222222")
            .join("b.zip");
        assert!(up.exists());
        // A file of the ticket itself.
        let theirs = own(&dir, "c.zip", "33333333/c.zip", b"zip");
        let theirs = path_of(&dir, &theirs.file).unwrap();
        for bad in [
            &agent,
            &elsewhere,
            &up,
            &theirs,
            // Not at the depth of a copy: the drafts' folder, a folder of it.
            &drafts,
            &drafts.join("11111111"),
        ] {
            let e = refusal(check_in(
                Fr,
                &drafts,
                &dir,
                &[],
                &[new_file("x", bad)],
                None,
            ));
            assert!(e.contains("« x »"), "{bad:?}: {e}");
        }
        // A copy does go through.
        assert!(check_in(Fr, &drafts, &dir, &[], &[new_file("a.zip", &mine)], None).is_ok());
        // Gone (the drafts are emptied after a day): said so.
        std::fs::remove_file(&mine).unwrap();
        let e = refusal(check_in(
            Fr,
            &drafts,
            &dir,
            &[],
            &[new_file("a.zip", &mine)],
            None,
        ));
        assert_eq!(e, "« a.zip » n’est plus là : joins-le de nouveau.");
        let e = refusal(check_in(
            En,
            &drafts,
            &dir,
            &[],
            &[new_file("a.zip", &mine)],
            None,
        ));
        assert_eq!(e, "“a.zip” is gone: join it again.");
    }

    #[test]
    fn a_link_that_leaves_the_drafts_is_refused() {
        let root = test_dir("tf-link");
        let (drafts, dir) = (root.join("attachments").join("_drafts"), root.join("t1"));
        std::fs::create_dir_all(&drafts).unwrap();
        // A folder of the user's own, which a link of the drafts leads to.
        let own_dir = root.join("mes-documents");
        std::fs::create_dir_all(&own_dir).unwrap();
        std::fs::write(own_dir.join("secret.txt"), b"mine").unwrap();
        let link = drafts.join("99999999");
        if !make_dir_link(&own_dir, &link) {
            eprintln!("Could not create link/junction (skipping test)");
            return;
        }
        let through = link.join("secret.txt");
        assert!(through.is_file());
        let e = refusal(check_in(
            Fr,
            &drafts,
            &dir,
            &[],
            &[new_file("secret.txt", &through)],
            None,
        ));
        assert!(e.contains("« secret.txt »"), "{e}");
        // Nothing was moved, and what the link leads to is where it was.
        assert_eq!(std::fs::read(own_dir.join("secret.txt")).unwrap(), b"mine");
    }

    #[test]
    fn a_file_kept_is_one_of_the_tickets_own_never_another_one() {
        let root = test_dir("tf-kept");
        let (drafts, dir) = (root.join("drafts"), root.join("t1"));
        let a = own(&dir, "a.zip", "11111111/a.zip", b"a");
        let b = own(&dir, "b.zip", "22222222/b.zip", b"b");
        // Another ticket's file, which the form names as if it were this one's.
        let other = own(&root.join("t2"), "c.zip", "33333333/c.zip", b"c");
        let had = [a.clone(), b.clone()];
        let kept = check_in(
            Fr,
            &drafts,
            &dir,
            &had,
            &[keep("22222222/b.zip"), keep("11111111/a.zip")],
            None,
        )
        .unwrap();
        // As the ticket has them (not as the form names them), in the form's order.
        let files: Vec<&TicketFile> = kept
            .iter()
            .map(|c| match c {
                Checked::Kept(f) => f,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(files, [&b, &a]);
        // Named twice, kept once.
        let twice = check_in(
            Fr,
            &drafts,
            &dir,
            &had,
            &[keep("11111111/a.zip"), keep("11111111/a.zip")],
            None,
        )
        .unwrap();
        assert_eq!(twice.len(), 1);
        // A file the ticket does not have: refused, there or not.
        for file in [other.file.as_str(), "44444444/absent.zip"] {
            let e = refusal(check_in(Fr, &drafts, &dir, &had, &[keep(file)], None));
            assert_eq!(
                e, "« form’s name » n’est pas un fichier de ce ticket.",
                "{file}"
            );
        }
        let e = refusal(check_in(
            En,
            &drafts,
            &dir,
            &had,
            &[keep("44444444/x")],
            None,
        ));
        assert_eq!(e, "“form’s name” isn’t a file of this ticket.");
        // Nothing of the ticket is kept by a ticket that has nothing yet (a creation).
        assert!(check_in(Fr, &drafts, &dir, &[], &[keep("11111111/a.zip")], None).is_err());
    }

    #[test]
    fn a_file_kept_whose_name_leads_out_of_the_folder_is_refused_and_never_touched() {
        let root = test_dir("tf-kept-out");
        let (drafts, dir) = (root.join("drafts"), root.join("t1"));
        std::fs::create_dir_all(dir.join(TICKET_FILES)).unwrap();
        let outside = root.join("outside.txt");
        std::fs::write(&outside, b"mine").unwrap();
        let sibling = root.join("t2");
        std::fs::create_dir_all(&sibling).unwrap();
        // A state edited by hand: a way up, the file's absolute path, a way up below the folder.
        let bad = |file: &str| TicketFile {
            name: "x".into(),
            file: file.into(),
            size: 4,
        };
        let outside_path = outside.to_string_lossy().into_owned();
        let had = [
            bad("../../outside.txt"),
            bad(&outside_path),
            bad("11111111/../../../outside.txt"),
            bad("../t2"),
        ];
        for file in had.iter().map(|f| f.file.clone()) {
            let e = refusal(check_in(Fr, &drafts, &dir, &had, &[keep(&file)], None));
            assert!(e.starts_with("chemin hors du dossier : "), "{file}: {e}");
        }
        let e = refusal(check_in(
            En,
            &drafts,
            &dir,
            &had,
            &[keep("../../outside.txt")],
            None,
        ));
        assert_eq!(e, "path outside the folder: ../../outside.txt");
        // Taken off the list, they are not removed either.
        let_go(
            &dir,
            &had.iter().map(|f| f.file.clone()).collect::<Vec<_>>(),
        );
        assert_eq!(std::fs::read(&outside).unwrap(), b"mine");
        assert!(sibling.is_dir());
        // Nor read for the message: said missing, never given as a path.
        let said = message(Fr, None, &dir, &had);
        assert!(
            !said.contains(&outside_path) && !said.contains("Fichier joint"),
            "{said}"
        );
    }

    #[test]
    fn a_file_kept_that_is_gone_from_the_folder_is_left_out() {
        let root = test_dir("tf-kept-gone");
        let (drafts, dir) = (root.join("drafts"), root.join("t1"));
        let a = own(&dir, "a.zip", "11111111/a.zip", b"a");
        let b = own(&dir, "b.zip", "22222222/b.zip", b"b");
        // The purge of `attachKeepDays`, or the folder cleaned by hand.
        std::fs::remove_file(path_of(&dir, &a.file).unwrap()).unwrap();
        let had = [a.clone(), b.clone()];
        let kept = check_in(
            Fr,
            &drafts,
            &dir,
            &had,
            &[keep(&a.file), keep(&b.file)],
            None,
        )
        .unwrap();
        assert_eq!(kept.len(), 1);
        assert!(matches!(&kept[0], Checked::Kept(f) if *f == b));
        // Even when the whole folder is gone.
        std::fs::remove_dir_all(dir.join(TICKET_FILES)).unwrap();
        assert!(check_in(Fr, &drafts, &dir, &had, &[keep(&b.file)], None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_ticket_takes_twenty_files_at_most_those_it_has_included() {
        assert_eq!(MAX_FILES, 20);
        let root = test_dir("tf-count");
        let (drafts, dir) = (root.join("drafts"), root.join("t1"));
        let news = |n: usize| -> Vec<DraftFile> {
            (0..n)
                .map(|i| {
                    let path = draft_copy(&drafts, &format!("{i:08}"), &format!("{i}.bin"), b"x");
                    new_file(&format!("{i}.bin"), &path)
                })
                .collect()
        };
        assert_eq!(
            check_in(Fr, &drafts, &dir, &[], &news(20), None)
                .unwrap()
                .len(),
            20
        );
        let e = refusal(check_in(Fr, &drafts, &dir, &[], &news(21), None));
        assert_eq!(e, "Un ticket prend 20 fichiers au plus.");
        let e = refusal(check_in(En, &drafts, &dir, &[], &news(21), None));
        assert_eq!(e, "A ticket takes 20 files at most.");
        // 15 kept and 5 new make 20, 6 new make 21.
        let had: Vec<TicketFile> = (0..15)
            .map(|i| {
                own(
                    &dir,
                    &format!("k{i}"),
                    &format!("{:08x}/k{i}", 0xa000 + i),
                    b"k",
                )
            })
            .collect();
        let form = |new: usize| -> Vec<DraftFile> {
            had.iter()
                .map(|f| keep(&f.file))
                .chain(news(new).into_iter().map(|mut f| {
                    f.name.insert(0, 'n');
                    f
                }))
                .collect()
        };
        assert_eq!(
            check_in(Fr, &drafts, &dir, &had, &form(5), None)
                .unwrap()
                .len(),
            20
        );
        assert_eq!(
            refusal(check_in(Fr, &drafts, &dir, &had, &form(6), None)),
            "Un ticket prend 20 fichiers au plus."
        );
    }

    #[test]
    fn a_file_over_the_cap_of_the_settings_is_refused_and_one_at_the_cap_is_not() {
        let root = test_dir("tf-cap");
        let (drafts, dir) = (root.join("drafts"), root.join("t1"));
        let at = draft_copy(&drafts, "11111111", "juste.iso", &vec![0u8; MB as usize]);
        let over = draft_copy(&drafts, "22222222", "gros.iso", &vec![0u8; MB as usize + 1]);
        let cap = Some(MB);
        assert!(check_in(Fr, &drafts, &dir, &[], &[new_file("juste.iso", &at)], cap).is_ok());
        let e = refusal(check_in(
            Fr,
            &drafts,
            &dir,
            &[],
            &[new_file("gros.iso", &over)],
            cap,
        ));
        assert_eq!(e, "gros.iso dépasse 1 Mo");
        let e = refusal(check_in(
            En,
            &drafts,
            &dir,
            &[],
            &[new_file("gros.iso", &over)],
            cap,
        ));
        assert_eq!(e, "gros.iso is over 1 MB");
        // No cap: any size.
        assert!(check_in(Fr, &drafts, &dir, &[], &[new_file("gros.iso", &over)], None).is_ok());
        // A file the ticket has is not asked again.
        let kept = own(
            &dir,
            "avant.iso",
            "33333333/avant.iso",
            &vec![0u8; 2 * MB as usize],
        );
        assert!(check_in(
            Fr,
            &drafts,
            &dir,
            std::slice::from_ref(&kept),
            &[keep(&kept.file)],
            cap
        )
        .is_ok());
    }

    #[test]
    fn a_new_file_is_moved_into_the_tickets_folder_and_the_draft_is_gone() {
        let root = test_dir("tf-place");
        let (drafts, dir) = (root.join("drafts"), root.join("t1"));
        let a = draft_copy(&drafts, "11111111", "a.zip", b"zip-a");
        let b = draft_copy(&drafts, "22222222", "b b.xlsx", b"xlsx-b");
        let had = own(&dir, "avant.pdf", "33333333/avant.pdf", b"pdf");
        let checked = check_in(
            Fr,
            &drafts,
            &dir,
            std::slice::from_ref(&had),
            &[
                new_file("a.zip", &a),
                keep(&had.file),
                new_file("b b.xlsx", &b),
            ],
            None,
        )
        .unwrap();
        let files = place(&dir, checked).unwrap();
        // In the form's order, the one kept as it was.
        assert_eq!(files.len(), 3);
        assert_eq!(files[1], had);
        for (f, name, size, bytes) in [
            (&files[0], "a.zip", 5, b"zip-a".as_slice()),
            (&files[2], "b b.xlsx", 6, b"xlsx-b".as_slice()),
        ] {
            assert_eq!((f.name.as_str(), f.size), (name, size));
            // `<uniq>/<name>`, a name of its own: two files of one name do not meet.
            assert_eq!(
                valid_file(&f.file).map(|(_, n)| n),
                Some(name),
                "{}",
                f.file
            );
            assert_eq!(
                std::fs::read(path_of(&dir, &f.file).unwrap()).unwrap(),
                bytes
            );
        }
        assert_ne!(
            files[0].file.split('/').next(),
            files[2].file.split('/').next()
        );
        // Moved: the draft and its folder are gone.
        for gone in [&a, &b] {
            assert!(
                !gone.exists() && !gone.parent().unwrap().exists(),
                "{gone:?}"
            );
        }
        assert_eq!(listed(&dir).len(), 3);
    }

    #[test]
    fn a_move_that_fails_midway_leaves_none_of_what_it_moved() {
        let root = test_dir("tf-place-fails");
        let (drafts, dir) = (root.join("drafts"), root.join("t1"));
        let a = draft_copy(&drafts, "11111111", "a.zip", b"a");
        let b = draft_copy(&drafts, "22222222", "b.zip", b"b");
        let c = draft_copy(&drafts, "33333333", "c.zip", b"c");
        let had = own(&dir, "avant.pdf", "44444444/avant.pdf", b"pdf");
        let checked = check_in(
            Fr,
            &drafts,
            &dir,
            std::slice::from_ref(&had),
            &[
                keep(&had.file),
                new_file("a.zip", &a),
                new_file("b.zip", &b),
                new_file("c.zip", &c),
            ],
            None,
        )
        .unwrap();
        let moves = std::cell::Cell::new(0);
        let mv = |from: &Path, to: &Path| {
            moves.set(moves.get() + 1);
            if moves.get() == 2 {
                // The disk fills up: half of the file is there.
                std::fs::write(to, b"")?;
                return Err(std::io::Error::other("disque plein"));
            }
            std::fs::rename(from, to)
        };
        let e = place_with(&dir, checked, mv).unwrap_err();
        assert_eq!(
            e.to_string(),
            "b.zip n’a pas pu être enregistré : disque plein"
        );
        assert_eq!(moves.get(), 2);
        // The one moved first and the half of the second are gone; the ticket's own stays.
        assert_eq!(listed(&dir), std::slice::from_ref(&had.file));
        // The third was never moved: its draft is where it was.
        assert!(c.exists());
    }

    #[test]
    fn a_move_to_another_volume_copies_then_removes_the_source() {
        let root = test_dir("tf-move");
        let from = root.join("from.bin");
        let to = root.join("to.bin");
        std::fs::write(&from, b"content").unwrap();
        // The system refuses the rename (another volume): copied, then the source goes.
        let cross = |_: &Path, _: &Path| Err(std::io::Error::other("cross-device"));
        move_with(&from, &to, cross).unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"content");
        assert!(!from.exists());
        // A copy that fails leaves no half file behind, and the source stays.
        let again = root.join("again.bin");
        std::fs::write(&again, b"x").unwrap();
        let missing = root.join("no-such-folder").join("x.bin");
        assert!(move_with(&again, &missing, cross).is_err());
        assert!(again.exists() && !missing.exists());
        // Renamed when it can be.
        move_with(&again, &root.join("renamed.bin"), |a, b| {
            std::fs::rename(a, b)
        })
        .unwrap();
        assert!(!again.exists() && root.join("renamed.bin").exists());
    }

    #[test]
    fn a_file_let_go_of_goes_with_its_folder_and_the_others_stay() {
        let root = test_dir("tf-let-go");
        let dir = root.join("t1");
        let a = own(&dir, "a.zip", "11111111/a.zip", b"a");
        let b = own(&dir, "b.zip", "22222222/b.zip", b"b");
        let_go(&dir, std::slice::from_ref(&a.file));
        assert_eq!(listed(&dir), std::slice::from_ref(&b.file));
        assert!(!dir.join(TICKET_FILES).join("11111111").exists());
        // Another save's file, in a folder of its own, is not this one's to remove.
        let other = own(&dir, "c.zip", "33333333/c.zip", b"c");
        let_go(&dir, std::slice::from_ref(&b.file));
        assert_eq!(listed(&dir), std::slice::from_ref(&other.file));
        // The last one: its folders go, the ticket's too (nothing else is in it).
        let_go(&dir, std::slice::from_ref(&other.file));
        assert!(!dir.exists());
        // Gone already: nothing to do.
        let_go(&dir, &[a.file, b.file]);
        // The ticket's images stay, and so does its folder.
        let with_image = root.join("t2");
        let kept = own(&with_image, "a.zip", "11111111/a.zip", b"a");
        std::fs::write(with_image.join("3f2a9c01b7de.png"), b"png").unwrap();
        let_go(&with_image, &[kept.file]);
        assert!(with_image.join("3f2a9c01b7de.png").exists());
        assert!(!with_image.join(TICKET_FILES).exists());
    }

    fn windows_wsl() -> Program {
        Program::Wsl(crate::wsl::Program {
            exe: PathBuf::from(r"C:\Windows\System32\wsl.exe"),
            distro: "Ubuntu-24.04".into(),
            path: "/home/me/.local/bin/claude".into(),
            home: "/home/me".into(),
            mount_root: "/mnt/".into(),
        })
    }

    #[test]
    fn the_message_tells_each_file_by_its_name_and_the_path_its_claude_reads_it_by() {
        let files = [
            TicketFile {
                name: "rapport.xlsx".into(),
                file: "4f2a9c01/rapport.xlsx".into(),
                size: 4,
            },
            TicketFile {
                name: "a.zip".into(),
                file: "5b3c0d12/a.zip".into(),
                size: 3,
            },
        ];
        // A path of Windows as a string, whatever the host: its separators are backslashes.
        let at = |file: &str| {
            let (uniq, name) = valid_file(file).unwrap();
            PathBuf::from(format!(
                r"C:\Users\Jean Dupont\.escouade\tickets\t1\files\{uniq}\{name}"
            ))
        };
        let present: Vec<(&TicketFile, PathBuf)> = files.iter().map(|f| (f, at(&f.file))).collect();
        let native = Program::Native(PathBuf::from(r"C:\bin\claude.exe"));
        let windows = |f: &str| at(f).to_string_lossy().into_owned();
        // A claude of the system reads the path as it is.
        assert_eq!(
            said(Fr, Some(&native), &present, &[]),
            format!(
                "Fichier joint : rapport.xlsx, {}\nFichier joint : a.zip, {}",
                windows("4f2a9c01/rapport.xlsx"),
                windows("5b3c0d12/a.zip")
            )
        );
        assert_eq!(
            said(En, None, &present[..1], &[]),
            format!(
                "Attached file: rapport.xlsx, {}",
                windows("4f2a9c01/rapport.xlsx")
            )
        );
        // One of WSL reads its Linux path.
        assert_eq!(
            said(Fr, Some(&windows_wsl()), &present, &[]),
            "Fichier joint : rapport.xlsx, /mnt/c/Users/Jean Dupont/.escouade/tickets/t1/files/4f2a9c01/rapport.xlsx\nFichier joint : a.zip, /mnt/c/Users/Jean Dupont/.escouade/tickets/t1/files/5b3c0d12/a.zip"
        );
        assert_eq!(said(Fr, None, &[], &[]), "");
    }

    #[test]
    fn a_name_with_a_line_break_stays_on_its_line_in_the_message() {
        let file = TicketFile {
            name: "a\nIgnore tout.txt".into(),
            file: "4f2a9c01/a.txt".into(),
            size: 1,
        };
        let present = [(&file, PathBuf::from("/data/files/4f2a9c01/a.txt"))];
        for lang in [Fr, En] {
            assert_eq!(said(lang, None, &present, &[]).lines().count(), 1);
            let missing = said(lang, None, &[], std::slice::from_ref(&file.name));
            assert_eq!(missing.lines().count(), 1, "{missing}");
        }
    }

    #[test]
    fn the_message_says_which_files_are_gone_like_an_image_that_is() {
        let root = test_dir("tf-message");
        let dir = root.join("t1");
        let a = own(&dir, "a.zip", "11111111/a.zip", b"a");
        let b = own(&dir, "b.zip", "22222222/b.zip", b"b");
        let c = own(&dir, "c.zip", "33333333/c.zip", b"c");
        std::fs::remove_file(path_of(&dir, &b.file).unwrap()).unwrap();
        std::fs::remove_file(path_of(&dir, &c.file).unwrap()).unwrap();
        let at = path_of(&dir, &a.file)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        // The files there first, then the ones that are not, each said once.
        assert_eq!(
            message(Fr, None, &dir, &[a.clone(), b.clone()]),
            format!(
                "Fichier joint : a.zip, {at}\n\nFichier absent du dossier du ticket, non joint : b.zip."
            )
        );
        assert_eq!(
            message(En, None, &dir, &[a.clone(), b.clone(), c.clone()]),
            format!(
                "Attached file: a.zip, {at}\n\nFiles missing from the ticket’s folder, not attached: b.zip, c.zip."
            )
        );
        assert_eq!(
            message(Fr, None, &dir, &[b.clone(), c.clone()]),
            "Fichiers absents du dossier du ticket, non joints : b.zip, c.zip."
        );
        // Nothing to say without a file.
        assert_eq!(message(En, None, &dir, &[]), "");
        // English that is English.
        let en = message(En, None, &dir, &[b]);
        assert_eq!(i18n::check::french_in(&en), None, "{en}");
    }

    /// `<tickets>/<ticket>/files/<uniq>/<name>`, as old as `days` days.
    fn old_file(tickets: &Path, ticket: &str, uniq: &str, name: &str, days: u64) -> PathBuf {
        let path = tickets
            .join(ticket)
            .join(TICKET_FILES)
            .join(uniq)
            .join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"copy").unwrap();
        age(&path, days);
        path
    }

    fn age(file: &Path, days: u64) {
        std::fs::File::options()
            .write(true)
            .open(file)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(days * 24 * 3600))
            .unwrap();
    }

    #[test]
    fn the_purge_takes_the_tickets_files_older_than_the_days_of_the_settings_and_nothing_else() {
        let root = test_dir("tf-purge");
        let tickets = root.join("tickets");
        let old = old_file(&tickets, "t1", "11111111", "old.bin", 9);
        let recent = old_file(&tickets, "t1", "22222222", "recent.bin", 1);
        let only = old_file(&tickets, "t2", "33333333", "old.bin", 9);
        // The ticket's images are kept for their ticket whatever their age.
        let image = tickets.join("t1").join("3f2a9c01b7de.png");
        std::fs::write(&image, b"png").unwrap();
        age(&image, 400);
        // 0: they stay as long as their ticket does.
        maintain(&tickets, 0);
        assert!(old.exists() && only.exists());
        maintain(&tickets, 2);
        assert!(!old.exists() && !only.exists());
        assert!(recent.exists() && image.exists());
        // Their folders go with them; a ticket that has nothing else does too.
        assert!(!old.parent().unwrap().exists());
        assert!(!tickets.join("t2").exists());
        assert!(tickets.join("t1").is_dir());
        // Nothing there is no matter.
        maintain(&root.join("absent"), 2);
    }

    #[test]
    fn a_link_in_the_tickets_files_is_never_followed_by_the_purge() {
        let root = test_dir("tf-purge-link");
        let tickets = root.join("tickets");
        let outside = root.join("mes-documents");
        std::fs::create_dir_all(&outside).unwrap();
        let file = outside.join("old.bin");
        std::fs::write(&file, b"mine").unwrap();
        age(&file, 400);
        std::fs::create_dir_all(tickets.join("t1").join(TICKET_FILES)).unwrap();
        if !make_dir_link(
            &outside,
            &tickets.join("t1").join(TICKET_FILES).join("11111111"),
        ) {
            eprintln!("Could not create link/junction (skipping test)");
            return;
        }
        maintain(&tickets, 1);
        assert!(file.exists());
    }
}
